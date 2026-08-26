use std::env;

use anyhow::{Context, Result};

#[derive(Clone, Debug)]
pub struct Config {
    pub app_name:      String,
    pub port:          u16,
    pub public_origin: String,
    pub database_url:  String,
    pub redis_url:     String,
    pub admin_ips:     Vec<String>,
}

impl Config {
    pub fn from_env(app_name: &str) -> Result<Self> {
        let port = env::var("PORT").ok().and_then(|v| v.parse().ok()).unwrap_or(3000);
        let public_origin = env::var("PUBLIC_ORIGIN").context("PUBLIC_ORIGIN required")?;
        let database_url = env::var("DATABASE_URL").context("DATABASE_URL required")?;
        let redis_url = env::var("REDIS_URL").context("REDIS_URL required")?;
        let admin_ips = env::var("ADMIN_IPS").ok().map_or_else(Vec::new, |s| {
            s.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect()
        });
        Ok(Self {
            app_name: app_name.to_string(),
            port,
            public_origin,
            database_url,
            redis_url,
            admin_ips,
        })
    }
}
