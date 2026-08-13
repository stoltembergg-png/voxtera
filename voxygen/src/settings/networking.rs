use hashbrown::HashSet;
use serde::{Deserialize, Serialize};

/// The default multiplayer endpoint shipped with Voxtera.
pub const DEFAULT_SERVER_ADDRESS: &str = "15.228.166.136:14004";

const LEGACY_SERVER_ADDRESSES: [&str; 2] = [
    "ec2-15-229-9-223.sa-east-1.compute.amazonaws.com:14004",
    "15.229.9.223:14004",
];

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
            servers: vec![DEFAULT_SERVER_ADDRESS.to_string()],
            default_server: DEFAULT_SERVER_ADDRESS.to_string(),
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

impl NetworkingSettings {
    pub fn migrate_legacy_server_addresses(&mut self) -> bool {
        let mut changed = false;
        for server in &mut self.servers {
            if LEGACY_SERVER_ADDRESSES.contains(&server.as_str()) {
                *server = DEFAULT_SERVER_ADDRESS.to_string();
                changed = true;
            }
        }
        if LEGACY_SERVER_ADDRESSES.contains(&self.default_server.as_str()) {
            self.default_server = DEFAULT_SERVER_ADDRESS.to_string();
            changed = true;
        }
        changed
    }
}

#[cfg(test)]
mod tests {
    use super::NetworkingSettings;

    #[test]
    fn default_server_points_to_the_current_voxtera_vps() {
        let settings = NetworkingSettings::default();

        assert_eq!(settings.default_server, super::DEFAULT_SERVER_ADDRESS);
        assert_eq!(settings.servers, vec![settings.default_server.clone()]);
        assert!(!settings.use_srv);
    }

    #[test]
    fn persisted_legacy_server_addresses_are_migrated() {
        let mut settings = NetworkingSettings {
            servers: vec![
                "ec2-15-229-9-223.sa-east-1.compute.amazonaws.com:14004".to_string(),
                "15.229.9.223:14004".to_string(),
            ],
            default_server: "ec2-15-229-9-223.sa-east-1.compute.amazonaws.com:14004".to_string(),
            ..NetworkingSettings::default()
        };

        assert!(settings.migrate_legacy_server_addresses());
        assert_eq!(settings.default_server, super::DEFAULT_SERVER_ADDRESS);
        assert_eq!(settings.servers, vec![
            super::DEFAULT_SERVER_ADDRESS.to_string(),
            super::DEFAULT_SERVER_ADDRESS.to_string(),
        ]);
        assert!(!settings.migrate_legacy_server_addresses());
    }
}
