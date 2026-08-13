//! Supabase Auth integration for Voxtera
//!
//! This module provides authentication through Supabase,
//! replacing the default Veloren auth server flow.
//!
//! Uses ureq (synchronous HTTP) for simplicity.
//! Only the anon public key is used client-side (safe to ship).

use serde::{Deserialize, Serialize};
use std::fmt;

/// Supabase configuration
pub struct SupabaseConfig {
    pub project_url: String,
    pub anon_key: String,
}

impl SupabaseConfig {
    /// Create config with the Voxtera Supabase project keys
    pub fn voxtera() -> Self {
        Self {
            project_url: "https://gcfavlnisyhdwseuvzpd.supabase.co".to_string(),
            anon_key: "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJpc3MiOiJzdXBhYmFzZSIsInJlZiI6ImdjZmF2bG5pc3loZHdzZXV2enBkIiwicm9sZSI6ImFub24iLCJpYXQiOjE3NzM3OTAwMTYsImV4cCI6MjA4OTM2NjAxNn0.64fur_GhfGq9Ksntv7shy7eTT5sC9XDsJxbiOgetEdc".to_string(),
        }
    }
}

/// Supabase auth response for signup/signin
#[derive(Debug, Serialize, Deserialize)]
pub struct AuthResponse {
    pub access_token: Option<String>,
    pub token_type: Option<String>,
    pub expires_in: Option<i64>,
    pub expires_at: Option<i64>,
    pub refresh_token: Option<String>,
    pub user: Option<User>,
    pub error: Option<String>,
    pub error_description: Option<String>,
}

/// Supabase user data
#[derive(Debug, Serialize, Deserialize)]
pub struct User {
    pub id: String,
    pub aud: String,
    pub role: String,
    pub email: Option<String>,
    pub email_confirmed_at: Option<String>,
    pub phone: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// Error type for Supabase auth operations
#[derive(Debug)]
pub enum SupabaseAuthError {
    NetworkError(String),
    AuthError(String),
    InvalidResponse(String),
    EmailNotConfirmed,
    UserAlreadyRegistered,
    InvalidCredentials,
}

impl fmt::Display for SupabaseAuthError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NetworkError(msg) => write!(f, "Network error: {}", msg),
            Self::AuthError(msg) => write!(f, "Auth error: {}", msg),
            Self::InvalidResponse(msg) => write!(f, "Invalid response: {}", msg),
            Self::EmailNotConfirmed => write!(f, "Email not confirmed"),
            Self::UserAlreadyRegistered => write!(f, "User already registered"),
            Self::InvalidCredentials => write!(f, "Invalid credentials"),
        }
    }
}

/// Supabase auth client — uses ureq (sync) for simplicity
pub struct SupabaseAuthClient {
    config: SupabaseConfig,
}

impl SupabaseAuthClient {
    pub fn new(config: SupabaseConfig) -> Self { Self { config } }

    /// Create client with default Voxtera config
    pub fn voxtera() -> Self { Self::new(SupabaseConfig::voxtera()) }

    /// Sign up a new user with email, password and username
    pub fn sign_up(
        &self,
        email: &str,
        password: &str,
        username: &str,
    ) -> Result<AuthResponse, SupabaseAuthError> {
        let url = format!("{}/auth/v1/signup", self.config.project_url);

        let body = serde_json::json!({
            "email": email,
            "password": password,
            "data": {
                "username": username
            }
        });

        let response = ureq::post(&url)
            .set("Content-Type", "application/json")
            .set("apikey", &self.config.anon_key)
            .set("Authorization", &format!("Bearer {}", self.config.anon_key))
            .send_json(&body)
            .map_err(|e| SupabaseAuthError::NetworkError(e.to_string()))?;

        self.handle_response(response)
    }

    /// Sign in an existing user with email and password
    pub fn sign_in(&self, email: &str, password: &str) -> Result<AuthResponse, SupabaseAuthError> {
        let url = format!(
            "{}/auth/v1/token?grant_type=password",
            self.config.project_url
        );

        let body = serde_json::json!({
            "email": email,
            "password": password
        });

        let response = ureq::post(&url)
            .set("Content-Type", "application/json")
            .set("apikey", &self.config.anon_key)
            .set("Authorization", &format!("Bearer {}", self.config.anon_key))
            .send_json(&body)
            .map_err(|e| SupabaseAuthError::NetworkError(e.to_string()))?;

        self.handle_response(response)
    }

    fn handle_response(&self, response: ureq::Response) -> Result<AuthResponse, SupabaseAuthError> {
        let status = response.status();

        let body_str = response
            .into_string()
            .map_err(|e| SupabaseAuthError::NetworkError(e.to_string()))?;

        let auth_response: AuthResponse = serde_json::from_str(&body_str)
            .map_err(|e| SupabaseAuthError::InvalidResponse(format!("{}: {}", e, body_str)))?;

        if let Some(error) = &auth_response.error {
            return match error.as_str() {
                "invalid_grant" => Err(SupabaseAuthError::InvalidCredentials),
                "email_not_confirmed" => Err(SupabaseAuthError::EmailNotConfirmed),
                _ => Err(SupabaseAuthError::AuthError(
                    auth_response
                        .error_description
                        .clone()
                        .unwrap_or_else(|| error.clone()),
                )),
            };
        }

        if status == 200 || status == 201 {
            Ok(auth_response)
        } else {
            Err(SupabaseAuthError::AuthError(format!(
                "HTTP {}: {}",
                status, body_str
            )))
        }
    }
}

/// Convert a Supabase UUID string to a Veloren-compatible UUID
pub fn supabase_uuid_to_veloren(supabase_uuid: &str) -> Option<authc::Uuid> {
    authc::Uuid::parse_str(supabase_uuid).ok()
}
