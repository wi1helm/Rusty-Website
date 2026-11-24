//! Application configuration.
//!
//! All application configuration is loaded and validated here.
//! Environment-specific defaults and security policies are also defined here.

use std::{env, net::IpAddr};
use thiserror::Error;

// ─────────────────────────────────────────────
// Application constants
// ─────────────────────────────────────────────

pub const APP_NAME: &str = "Rust App 4000";
pub const APP_YEAR: u16 = 2026;

pub const DEFAULT_LOGO_KEY: &str = "defaults/default_logo.webp";
pub const DEFAULT_BANNER_KEY: &str = "defaults/default_banner.webp";

// ─────────────────────────────────────────────
// Application environment
// ─────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppEnvironment {
    Development,
    Staging,
    Production,
}

impl AppEnvironment {
    fn parse(value: &str) -> Result<Self, String> {
        match value.to_lowercase().as_str() {
            "development" | "dev" => Ok(Self::Development),
            "staging" => Ok(Self::Staging),
            "production" | "prod" => Ok(Self::Production),
            _ => Err(format!("unknown environment '{value}'")),
        }
    }

    pub fn is_development(&self) -> bool {
        matches!(self, Self::Development)
    }
    pub fn is_staging(&self) -> bool {
        matches!(self, Self::Staging)
    }
    pub fn is_production(&self) -> bool {
        matches!(self, Self::Production)
    }
}

// ─────────────────────────────────────────────
// Main application configuration
// ─────────────────────────────────────────────

#[derive(Clone)]
pub struct AppConfig {
    pub environment: AppEnvironment,
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub cache: CacheConfig,
    pub object_storage: ObjectStorageConfig,
    pub cookies: CookieConfig,
    pub log_filter: String,
}

impl AppConfig {
    /// Load configuration from environment variables.
    ///
    /// All required variables are checked before returning.
    /// Missing or invalid variables are reported together.
    pub fn from_env() -> Result<Self, ConfigError> {
        let mut errors = Vec::new();

        // Application environment
        let environment = required("APP_ENV", &mut errors).and_then(|value| {
            match AppEnvironment::parse(&value) {
                Ok(environment) => Some(environment),
                Err(error) => {
                    errors.push(format!("APP_ENV: {error}"));
                    None
                }
            }
        });

        // Server
        let host = required("APP_HOST", &mut errors).and_then(|value| match value.parse::<IpAddr>() {
                Ok(host) => Some(host),
                Err(_) => {
                    errors.push(format!("APP_HOST: invalid IP address '{value}'"));
                    None
                }
            });

        let port = required("APP_PORT", &mut errors).and_then(|value| match value.parse::<u16>() {
            Ok(port) if port != 0 => Some(port),
            _ => {
                errors.push(format!("APP_PORT: invalid port '{value}'"));
                None
            }
        });

        // Database
        let database_url = required("DATABASE_URL", &mut errors);

        // Cache
        let cache_url = required("CACHE_URL", &mut errors);

        // Object storage
        let s3_endpoint = required("S3_ENDPOINT", &mut errors);
        let s3_bucket = required("S3_BUCKET", &mut errors);
        let s3_access_key = required("S3_ACCESS_KEY", &mut errors);
        let s3_secret_key = required("S3_SECRET_KEY", &mut errors);
        let s3_public_url = required("S3_PUBLIC_URL", &mut errors);

        // Logging
        let log_filter = env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string());

        // Stop if configuration is invalid.
        if !errors.is_empty() { return Err(ConfigError(errors)); }

        // Safe to unwrap: all required values were validated above.
        let environment = environment.unwrap();

        Ok(Self {
            environment,
            server: ServerConfig {
                host: host.unwrap(),
                port: port.unwrap(),
            },
            database: DatabaseConfig {
                url: database_url.unwrap(),
            },
            cache: CacheConfig {
                url: cache_url.unwrap(),
            },
            object_storage: ObjectStorageConfig {
                endpoint: s3_endpoint.unwrap(),
                bucket: s3_bucket.unwrap(),
                access_key: s3_access_key.unwrap(),
                secret_key: s3_secret_key.unwrap(),
                public_url: s3_public_url.unwrap(),
            },
            cookies: CookieConfig::for_environment(environment),
            log_filter,
        })
    }
}

// ─────────────────────────────────────────────
// Server
// ─────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct ServerConfig {
    pub host: IpAddr,
    pub port: u16,
}

// ─────────────────────────────────────────────
// Database
// ─────────────────────────────────────────────

#[derive(Clone)]
pub struct DatabaseConfig {
    pub url: String,
}

// ─────────────────────────────────────────────
// Cache
// ─────────────────────────────────────────────

#[derive(Clone)]
pub struct CacheConfig {
    pub url: String,
}

// ─────────────────────────────────────────────
// Object storage
// ─────────────────────────────────────────────

#[derive(Clone)]
pub struct ObjectStorageConfig {
    pub endpoint: String,
    pub bucket: String,
    pub access_key: String,
    pub secret_key: String,
    pub public_url: String,
}

// ─────────────────────────────────────────────
// Cookies
// ─────────────────────────────────────────────

#[derive(Debug, Clone, Copy)]
pub enum CookieSameSite {
    Strict,
    Lax,
    None,
}

#[derive(Debug, Clone)]
pub struct CookieConfig {
    pub secure: bool,
    pub http_only: bool,
    pub same_site: CookieSameSite,
}

impl CookieConfig {
    fn for_environment(environment: AppEnvironment) -> Self {
        Self {
            secure: !environment.is_development(),
            http_only: true,
            same_site: CookieSameSite::Lax,
        }
    }
}

// ─────────────────────────────────────────────
// Configuration errors
// ─────────────────────────────────────────────

#[derive(Debug, Error)]
#[error("Configuration validation failed:\n{}", .0.join("\n"))]
pub struct ConfigError(pub Vec<String>);

// ─────────────────────────────────────────────
// Environment helpers
// ─────────────────────────────────────────────

fn required(name: &str, errors: &mut Vec<String>) -> Option<String> {
    match env::var(name) {
        Ok(value) if !value.trim().is_empty() => Some(value),
        Ok(_) => {
            errors.push(format!("{name}: value cannot be empty"));
            None
        }
        Err(env::VarError::NotPresent) => {
            errors.push(format!("{name}: missing environment variable"));
            None
        }
        Err(env::VarError::NotUnicode(_)) => {
            errors.push(format!("{name}: value is not valid Unicode"));
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cookie_security_depends_on_environment() {
        assert!(!CookieConfig::for_environment(AppEnvironment::Development).secure);
        assert!(CookieConfig::for_environment(AppEnvironment::Staging).secure);
        assert!(CookieConfig::for_environment(AppEnvironment::Production).secure);
    }
}
