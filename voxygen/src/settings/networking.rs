use hashbrown::HashSet;
use serde::{Deserialize, Serialize};

/// `NetworkingSettings` stores server and networking settings.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct NetworkingSettings {
    pub username: String,
    pub servers: Vec<String>,
    pub default_server: String,
    pub trusted_auth_servers: HashSet<String>,
    pub use_srv: bool,
    pub use_quic: bool,
    pub validate_tls: bool,
    pub player_physics_behavior: bool,
    pub lossy_terrain_compression: bool,
    pub enable_discord_integration: bool,
}

impl Default for NetworkingSettings {
    fn default() -> Self {
        Self {
            username: "".to_string(),
            servers: vec!["ec2-15-229-9-223.sa-east-1.compute.amazonaws.com:14004".to_string()],
            default_server: "ec2-15-229-9-223.sa-east-1.compute.amazonaws.com:14004".to_string(),
            trusted_auth_servers: ["https://gcfavlnisyhdwseuvzpd.supabase.co"]
                .iter()
                .map(|s| s.to_string())
                .collect(),
            use_srv: false,
            use_quic: false,
            validate_tls: true,
            player_physics_behavior: false,
            lossy_terrain_compression: false,
            enable_discord_integration: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::NetworkingSettings;

    #[test]
    fn default_server_points_to_the_current_voxtera_vps() {
        let settings = NetworkingSettings::default();

        assert_eq!(
            settings.default_server,
            "ec2-15-229-9-223.sa-east-1.compute.amazonaws.com:14004"
        );
        assert_eq!(settings.servers, vec![settings.default_server.clone()]);
        assert!(!settings.use_srv);
    }
}
