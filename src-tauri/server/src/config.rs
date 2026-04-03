use std::env;

#[derive(Clone, Debug)]
pub struct Config {
    pub host: String,
    pub port: u16,
    pub database_url: String,
    pub jwt_secret: String,
    pub jwt_access_expiry_secs: i64,
    pub jwt_refresh_expiry_secs: i64,
    /// Seconds to hold a segment lock after the holder disconnects (grace period).
    /// Default 30s for production; tests can use 0.
    pub ws_lock_timeout_secs: u64,
    /// Comma-separated list of allowed CORS origins.
    /// Empty string or "*" means permissive (development only).
    pub allowed_origins: Vec<String>,
    /// Max login/register attempts per IP per minute.
    pub auth_rate_limit_per_min: u32,
}

impl Config {
    pub fn from_env() -> anyhow::Result<Self> {
        let allowed_origins = env::var("ALLOWED_ORIGINS")
            .unwrap_or_default()
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(String::from)
            .collect();

        Ok(Self {
            host: env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string()),
            port: env::var("PORT")
                .unwrap_or_else(|_| "8080".to_string())
                .parse()?,
            database_url: env::var("DATABASE_URL")
                .unwrap_or_else(|_| "sqlite://memoq.db".to_string()),
            jwt_secret: env::var("JWT_SECRET")
                .unwrap_or_else(|_| "change-me-in-production-secret-key".to_string()),
            jwt_access_expiry_secs: 30 * 60,        // 30 minutes
            jwt_refresh_expiry_secs: 7 * 24 * 3600, // 7 days
            ws_lock_timeout_secs: 30,
            allowed_origins,
            auth_rate_limit_per_min: env::var("AUTH_RATE_LIMIT_PER_MIN")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(10),
        })
    }
}
