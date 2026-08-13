use crate::{
    Client,
    settings::{AdminRecord, Ban, Banlist, WhitelistRecord, banlist::NormalizedIpAddr},
};
use authc::{AuthClient, AuthClientError, AuthToken, Uuid};
use chrono::Utc;
use common::comp::AdminRole;
use common_net::msg::RegisterError;
use hashbrown::HashMap;
use specs::Component;
use std::{str::FromStr, sync::Arc};
use tokio::{runtime::Runtime, sync::oneshot};
use tracing::{error, info};

/// Supabase JWT claims
#[derive(Debug, serde::Deserialize)]
struct SupabaseClaims {
    sub: String,
    role: String,
    #[serde(default)]
    user_metadata: Option<serde_json::Value>,
    #[serde(default)]
    email: Option<String>,
}

/// Configuration for Supabase auth (ES256 public key)
pub struct SupabaseConfig {
    pub decoding_key: jsonwebtoken::DecodingKey,
    pub kid: String,
}

/// Determines whether a user is banned
pub fn ban_applies(
    ban: &Ban,
    admin: Option<&AdminRecord>,
    now: chrono::DateTime<chrono::Utc>,
) -> bool {
    let exceeds_ban_role = |admin: &AdminRecord| {
        AdminRole::from(admin.role) >= AdminRole::from(ban.performed_by_role())
    };
    !ban.is_expired(now) && !admin.is_some_and(exceeds_ban_role)
}

fn derive_uuid(username: &str) -> Uuid {
    let mut state = 144066263297769815596495629667062367629;
    for byte in username.as_bytes() {
        state ^= *byte as u128;
        state = state.wrapping_mul(309485009821345068724781371);
    }
    Uuid::from_u128(state)
}

pub fn derive_singleplayer_uuid() -> Uuid { derive_uuid("singleplayer") }

pub struct PendingLogin {
    pending_r: oneshot::Receiver<Result<(String, Uuid), RegisterError>>,
}

impl PendingLogin {
    pub(crate) fn new_success(username: String, uuid: Uuid) -> Self {
        let (pending_s, pending_r) = oneshot::channel();
        let _ = pending_s.send(Ok((username, uuid)));
        Self { pending_r }
    }
}

impl Component for PendingLogin {
    type Storage = specs::DenseVecStorage<Self>;
}

pub struct LoginProvider {
    runtime: Arc<Runtime>,
    auth_server: Option<Arc<AuthClient>>,
    supabase: Option<SupabaseConfig>,
    supabase_config_error: Option<String>,
}

impl LoginProvider {
    fn load_supabase_config() -> Result<SupabaseConfig, String> {
        let required_env = |name: &str| {
            std::env::var(name)
                .ok()
                .filter(|value| !value.is_empty())
                .ok_or_else(|| format!("missing required Supabase configuration: {name}"))
        };

        let kid = required_env("SUPABASE_KID")?;
        let x = required_env("SUPABASE_KEY_X")?;
        let y = required_env("SUPABASE_KEY_Y")?;
        let decoding_key = jsonwebtoken::DecodingKey::from_ec_components(&x, &y)
            .map_err(|_| "invalid Supabase EC public key configuration".to_string())?;

        Ok(SupabaseConfig { decoding_key, kid })
    }

    pub fn new(auth_addr: Option<String>, runtime: Arc<Runtime>) -> Self {
        tracing::trace!(?auth_addr, "Starting LoginProvider");

        let is_supabase = auth_addr
            .as_ref()
            .map(|addr| addr.contains("supabase.co"))
            .unwrap_or(false);

        let auth_server = if is_supabase {
            None
        } else {
            auth_addr.map(|addr| {
                let (scheme, authority) = addr.split_once("://").expect("invalid auth url");
                let scheme = scheme
                    .parse::<authc::Scheme>()
                    .expect("invalid auth url scheme");
                let authority = authority
                    .parse::<authc::Authority>()
                    .expect("invalid auth url authority");
                Arc::new(AuthClient::new(scheme, authority).expect("insecure auth scheme"))
            })
        };

        // For Supabase, load the EC public key exclusively from the environment.
        let (supabase, supabase_config_error) = if is_supabase {
            match Self::load_supabase_config() {
                Ok(config) => {
                    info!("Supabase auth configured with ES256 public key");
                    (Some(config), None)
                },
                Err(error) => {
                    error!(%error, "Supabase authentication is unavailable");
                    (None, Some(error))
                },
            }
        } else {
            (None, None)
        };

        Self {
            runtime,
            auth_server,
            supabase,
            supabase_config_error,
        }
    }

    pub fn verify(&self, username_or_token: &str) -> PendingLogin {
        let (pending_s, pending_r) = oneshot::channel();

        if let Some(error) = &self.supabase_config_error {
            let _ = pending_s.send(Err(RegisterError::AuthError(error.clone())));
        } else if let Some(supabase_config) = &self.supabase {
            // Supabase JWT validation (ES256)
            let token = username_or_token.to_string();
            let decoding_key = &supabase_config.decoding_key;

            match Self::validate_supabase_token(&token, decoding_key) {
                Ok((username, uuid)) => {
                    let _ = pending_s.send(Ok((username, uuid)));
                },
                Err(e) => {
                    let _ = pending_s.send(Err(e));
                },
            }
        } else if let Some(srv) = &self.auth_server {
            let srv = Arc::clone(srv);
            let username_or_token = username_or_token.to_string();
            self.runtime.spawn(async move {
                let _ = pending_s.send(Self::query(srv, &username_or_token).await);
            });
        } else {
            let username = username_or_token;
            let uuid = derive_uuid(username);
            let _ = pending_s.send(Ok((username.to_string(), uuid)));
        }

        PendingLogin { pending_r }
    }

    /// Validate a Supabase JWT token using ES256 public key
    fn validate_supabase_token(
        token: &str,
        decoding_key: &jsonwebtoken::DecodingKey,
    ) -> Result<(String, Uuid), RegisterError> {
        use jsonwebtoken::{Algorithm, Validation, decode};

        let mut validation = Validation::new(Algorithm::ES256);
        validation.set_audience(&["authenticated"]);
        validation.validate_exp = true;

        let token_data =
            decode::<SupabaseClaims>(token, decoding_key, &validation).map_err(|e| {
                error!(?e, "Supabase token validation failed");
                RegisterError::AuthError(format!("Invalid token: {}", e))
            })?;

        let claims = token_data.claims;

        if claims.role != "authenticated" {
            return Err(RegisterError::AuthError(
                "Token is not for authenticated user".to_string(),
            ));
        }

        let uuid = Uuid::from_str(&claims.sub)
            .map_err(|_| RegisterError::AuthError("Invalid UUID in token".to_string()))?;

        let username = claims
            .user_metadata
            .as_ref()
            .and_then(|m| m.get("username"))
            .and_then(|v| v.as_str())
            .or(claims.email.as_deref())
            .unwrap_or(&claims.sub)
            .to_string();

        info!(?uuid, ?username, "Supabase token validated via ES256");

        Ok((username, uuid))
    }

    pub(crate) fn login<R>(
        pending: &mut PendingLogin,
        client: &Client,
        admins: &HashMap<Uuid, AdminRecord>,
        whitelist: &HashMap<Uuid, WhitelistRecord>,
        banlist: &Banlist,
        player_count_exceeded: impl FnOnce(String, Uuid) -> (bool, R),
        make_ip_ban_upgrade: impl FnOnce(NormalizedIpAddr, Uuid, String),
    ) -> Option<Result<R, RegisterError>> {
        match pending.pending_r.try_recv() {
            Ok(Err(e)) => Some(Err(e)),
            Ok(Ok((username, uuid))) => {
                let now = Utc::now();
                let ip = client
                    .connected_from_addr()
                    .socket_addr()
                    .map(|s| s.ip())
                    .map(NormalizedIpAddr::from);
                let admin = admins.get(&uuid);
                if let Some(ban) = banlist
                    .uuid_bans()
                    .get(&uuid)
                    .and_then(|ban_entry| ban_entry.current.action.ban())
                    .into_iter()
                    .chain(ip.and_then(|ip| {
                        banlist
                            .ip_bans()
                            .get(&ip)
                            .and_then(|ban_entry| ban_entry.current.action.ban())
                    }))
                    .find(|ban| ban_applies(ban, admin, now))
                {
                    if let Some(ip) = ip
                        && ban.upgrade_to_ip
                    {
                        make_ip_ban_upgrade(ip, uuid, username.clone());
                    }
                    return Some(Err(RegisterError::Banned(ban.info())));
                }
                if admin.is_none() && !whitelist.is_empty() && !whitelist.contains_key(&uuid) {
                    return Some(Err(RegisterError::NotOnWhitelist));
                }
                let (player_count_exceeded, res) = player_count_exceeded(username, uuid);
                if admin.is_none() && player_count_exceeded {
                    return Some(Err(RegisterError::TooManyPlayers));
                }
                Some(Ok(res))
            },
            Err(oneshot::error::TryRecvError::Closed) => {
                error!("channel got closed too early");
                Some(Err(RegisterError::AuthError(
                    "Internal Error verifying".to_string(),
                )))
            },
            Err(oneshot::error::TryRecvError::Empty) => None,
        }
    }

    async fn query(
        srv: Arc<AuthClient>,
        username_or_token: &str,
    ) -> Result<(String, Uuid), RegisterError> {
        info!(?username_or_token, "Validating token via authc");
        let token = AuthToken::from_str(username_or_token)
            .map_err(|e| RegisterError::AuthError(e.to_string()))?;
        match async {
            let uuid = srv.validate(token).await?;
            let username = srv.uuid_to_username(uuid).await?;
            let r: Result<_, AuthClientError> = Ok((username, uuid));
            r
        }
        .await
        {
            Err(e) => Err(RegisterError::AuthError(e.to_string())),
            Ok((username, uuid)) => Ok((username, uuid)),
        }
    }

    pub fn username_to_uuid(&self, username: &str) -> Result<Uuid, AuthClientError> {
        match &self.auth_server {
            Some(srv) => self.runtime.block_on(srv.username_to_uuid(&username)),
            None => Ok(derive_uuid(username)),
        }
    }

    pub fn uuid_to_username(
        &self,
        uuid: Uuid,
        fallback_alias: &str,
    ) -> Result<String, AuthClientError> {
        match &self.auth_server {
            Some(srv) => self.runtime.block_on(srv.uuid_to_username(uuid)),
            None => Ok(fallback_alias.into()),
        }
    }
}
