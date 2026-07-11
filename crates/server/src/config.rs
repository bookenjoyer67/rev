use serde::Deserialize;
use std::path::Path;

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct Config {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub node: NodeConfig,
    pub discovery: DiscoveryConfig,
    pub auth: AuthConfig,
    pub federation: FederationConfig,
    pub security: SecurityConfig,
    pub posts: PostsConfig,
    pub admin: AdminConfig,
    pub relay: RelayConfig,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct RelayConfig {
    pub enabled: bool,
    pub port: u16,
    pub bind_address: String,
    pub storage_path: String,
    pub max_clients_per_room: usize,
    pub external_url: Option<String>,
}

impl Default for RelayConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            port: 9001,
            bind_address: "0.0.0.0".into(),
            storage_path: "data/relay".into(),
            max_clients_per_room: 100,
            external_url: None,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct ServerConfig {
    pub bind_address: String,
    pub port: u16,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct DatabaseConfig {
    pub url: String,
    pub max_connections: u32,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct NodeConfig {
    pub name: String,
    pub description: String,
    pub public_url: Option<String>,
    pub location_name: Option<String>,
    pub location_lat: Option<f64>,
    pub location_lon: Option<f64>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct DiscoveryConfig {
    pub listed: bool,
    pub directory_url: Option<String>,
    pub directory_enabled: bool,
    pub registration_mode: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct AuthConfig {
    pub jwt_secret: String,
    pub token_lifetime_days: u32,
    pub max_registrations_per_hour: u32,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct FederationConfig {
    pub enabled: bool,
    pub domain: Option<String>,
    pub max_alliances: u32,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct SecurityConfig {
    pub max_posts_per_hour: u32,
    pub max_messages_per_hour: u32,
    pub max_matches_per_hour: u32,
    pub allowed_origins: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct PostsConfig {
    pub default_ttl_need_days: u32,
    pub default_ttl_offer_days: u32,
    pub default_ttl_resource_days: u32,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct AdminConfig {
    pub superadmin_keys: Vec<String>,
}

impl Default for PostsConfig {
    fn default() -> Self {
        Self {
            default_ttl_need_days: 7,
            default_ttl_offer_days: 14,
            default_ttl_resource_days: 0,
        }
    }
}

impl Default for AdminConfig {
    fn default() -> Self {
        Self {
            superadmin_keys: Vec::new(),
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            server: ServerConfig::default(),
            database: DatabaseConfig::default(),
            node: NodeConfig::default(),
            discovery: DiscoveryConfig::default(),
            auth: AuthConfig::default(),
            federation: FederationConfig::default(),
            security: SecurityConfig::default(),
            posts: PostsConfig::default(),
            admin: AdminConfig::default(),
            relay: RelayConfig::default(),
        }
    }
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            bind_address: "0.0.0.0".into(),
            port: 3000,
        }
    }
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            url: "postgres://komun:komun@localhost:5432/komun".into(),
            max_connections: 20,
        }
    }
}

impl Default for NodeConfig {
    fn default() -> Self {
        Self {
            name: "Komun Node".into(),
            description: "A community mutual aid server".into(),
            public_url: None,
            location_name: None,
            location_lat: None,
            location_lon: None,
        }
    }
}

impl Default for DiscoveryConfig {
    fn default() -> Self {
        Self {
            listed: false,
            directory_url: None,
            directory_enabled: false,
            registration_mode: "open".into(),
        }
    }
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            jwt_secret: "komun-dev-secret-change-in-production".into(),
            token_lifetime_days: 30,
            max_registrations_per_hour: 20,
        }
    }
}

impl Default for FederationConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            domain: None,
            max_alliances: 50,
        }
    }
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            max_posts_per_hour: 60,
            max_messages_per_hour: 200,
            max_matches_per_hour: 30,
            allowed_origins: "*".into(),
        }
    }
}

impl Config {
    pub fn load() -> anyhow::Result<Self> {
        let config_path = std::env::var("KOMUN_CONFIG")
            .unwrap_or_else(|_| "config.toml".into());

        let mut config = if Path::new(&config_path).exists() {
            let content = std::fs::read_to_string(&config_path)?;
            toml::from_str::<Config>(&content)?
        } else {
            Config::default()
        };

        config.apply_env_overrides();

        if std::env::var("JWT_SECRET").is_err() {
            tracing::warn!(
                "JWT_SECRET not set in environment. Using value from config.toml. \
                 Set JWT_SECRET env var for production deployments."
            );
        }

        if config.auth.jwt_secret.len() < 32 {
            tracing::error!(
                "jwt_secret is too short ({} chars). Must be at least 32 characters.",
                config.auth.jwt_secret.len()
            );
            return Err(anyhow::anyhow!("jwt_secret must be at least 32 characters"));
        }
        Ok(config)
    }

    fn apply_env_overrides(&mut self) {
        if let Ok(v) = std::env::var("KOMUN_BIND_ADDRESS") {
            self.server.bind_address = v;
        }
        if let Ok(v) = std::env::var("KOMUN_PORT") {
            if let Ok(p) = v.parse() { self.server.port = p; }
        }
        if let Ok(v) = std::env::var("DATABASE_URL") {
            self.database.url = v;
        }
        if let Ok(v) = std::env::var("KOMUN_NODE_NAME") {
            self.node.name = v;
        }
        if let Ok(v) = std::env::var("JWT_SECRET") {
            self.auth.jwt_secret = v;
        }
        if let Ok(v) = std::env::var("BIND_ADDR") {
            let parts: Vec<&str> = v.rsplitn(2, ':').collect();
            if parts.len() == 2 {
                if let Ok(p) = parts[0].parse() { self.server.port = p; }
                self.server.bind_address = parts[1].into();
            }
        }
    }

    pub fn bind_addr(&self) -> String {
        format!("{}:{}", self.server.bind_address, self.server.port)
    }

    pub fn public_url(&self) -> String {
        self.node.public_url.clone()
            .unwrap_or_else(|| format!("http://localhost:{}", self.server.port))
    }
}
