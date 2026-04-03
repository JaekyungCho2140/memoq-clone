use std::env;

#[derive(Clone, Debug)]
pub struct Config {
    pub host: String,
    pub port: u16,
    pub database_url: String,
    pub jwt_secret: String,
    pub jwt_access_expiry_secs: i64,
    pub jwt_refresh_expiry_secs: i64,
}

impl Config {
    pub fn from_env() -> anyhow::Result<Self> {
        Ok(Self {
            host: env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string()),
            port: env::var("PORT")
                .unwrap_or_else(|_| "8080".to_string())
                .parse()?,
            database_url: env::var("DATABASE_URL")
                .unwrap_or_else(|_| "sqlite://memoq.db".to_string()),
            jwt_secret: env::var("JWT_SECRET")
                .unwrap_or_else(|_| "change-me-in-production-secret-key".to_string()),
            jwt_access_expiry_secs: 30 * 60,       // 30 minutes
            jwt_refresh_expiry_secs: 7 * 24 * 3600, // 7 days
        })
    }
}
