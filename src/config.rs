//! Application configuration.
//!
//! All application configuration is loaded and validated here.
//! Environment-specific defaults and security policies are also defined here.

use std::{env, net::IpAddr, str::FromStr, time::Duration};
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
    pub rate_limits: RateLimitConfig,
    pub rate_limit_hmac_key: String,
    pub trusted_proxies: Vec<TrustedProxyNetwork>,
    pub argon2_max_concurrency: usize,
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
        let environment = required("APP_ENV", &mut errors).and_then(|value| match AppEnvironment::parse(&value) {
            Ok(environment) => Some(environment),
            Err(error) => {
                errors.push(format!("APP_ENV: {error}"));
                None
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

        // Public endpoint protections. The HMAC key is intentionally required
        // so deployments cannot accidentally store account identifiers unhashed.
        let rate_limit_hmac_key = required("RATE_LIMIT_HMAC_KEY", &mut errors).and_then(|key| {
            if key.len() >= 32 {
                Some(key)
            } else {
                errors.push("RATE_LIMIT_HMAC_KEY: must contain at least 32 bytes".to_owned());
                None
            }
        });
        let rate_limits = RateLimitConfig {
            login_ip_limit: positive_u64("LOGIN_IP_LIMIT", 10, &mut errors),
            login_ip_window: Duration::from_secs(positive_u64("LOGIN_IP_WINDOW_SECONDS", 60, &mut errors)),
            login_account_limit: positive_u64("LOGIN_ACCOUNT_LIMIT", 5, &mut errors),
            login_account_window: Duration::from_secs(positive_u64("LOGIN_ACCOUNT_WINDOW_SECONDS", 300, &mut errors)),
            login_ip_account_limit: positive_u64("LOGIN_IP_ACCOUNT_LIMIT", 5, &mut errors),
            login_ip_account_window: Duration::from_secs(positive_u64("LOGIN_IP_ACCOUNT_WINDOW_SECONDS", 300, &mut errors)),
            signup_ip_limit: positive_u64("SIGNUP_IP_LIMIT", 5, &mut errors),
            signup_ip_window: Duration::from_secs(positive_u64("SIGNUP_IP_WINDOW_SECONDS", 3600, &mut errors)),
            signup_email_limit: positive_u64("SIGNUP_EMAIL_LIMIT", 3, &mut errors),
            signup_email_window: Duration::from_secs(positive_u64("SIGNUP_EMAIL_WINDOW_SECONDS", 3600, &mut errors)),
            signup_daily_ip_limit: positive_u64("SIGNUP_DAILY_IP_LIMIT", 20, &mut errors),
            signup_daily_ip_window: Duration::from_secs(positive_u64("SIGNUP_DAILY_IP_WINDOW_SECONDS", 86400, &mut errors)),
        };
        let argon2_max_concurrency = positive_usize("ARGON2_MAX_CONCURRENCY", 4, &mut errors);
        let trusted_proxies = match env::var("TRUSTED_PROXY_CIDRS") {
            Ok(value) => value
                .split(',')
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .filter_map(|value| match TrustedProxyNetwork::from_str(value) {
                    Ok(network) => Some(network),
                    Err(error) => {
                        errors.push(format!("TRUSTED_PROXY_CIDRS: {error}"));
                        None
                    }
                })
                .collect(),
            Err(env::VarError::NotPresent) => Vec::new(),
            Err(env::VarError::NotUnicode(_)) => {
                errors.push("TRUSTED_PROXY_CIDRS: value is not valid Unicode".to_owned());
                Vec::new()
            }
        };

        // Logging
        let log_filter = env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string());

        // Stop if configuration is invalid.
        if !errors.is_empty() {
            return Err(ConfigError(errors));
        }

        // Safe to unwrap: all required values were validated above.
        let environment = environment.unwrap();

        Ok(Self {
            environment,
            server: ServerConfig { host: host.unwrap(), port: port.unwrap() },
            database: DatabaseConfig { url: database_url.unwrap() },
            cache: CacheConfig { url: cache_url.unwrap() },
            object_storage: ObjectStorageConfig { endpoint: s3_endpoint.unwrap(), bucket: s3_bucket.unwrap(), access_key: s3_access_key.unwrap(), secret_key: s3_secret_key.unwrap() },
            cookies: CookieConfig::for_environment(environment),
            rate_limits,
            rate_limit_hmac_key: rate_limit_hmac_key.unwrap(),
            trusted_proxies,
            argon2_max_concurrency,
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

#[derive(Clone)]
pub struct RateLimitConfig {
    pub login_ip_limit: u64,
    pub login_ip_window: Duration,
    pub login_account_limit: u64,
    pub login_account_window: Duration,
    pub login_ip_account_limit: u64,
    pub login_ip_account_window: Duration,
    pub signup_ip_limit: u64,
    pub signup_ip_window: Duration,
    pub signup_email_limit: u64,
    pub signup_email_window: Duration,
    pub signup_daily_ip_limit: u64,
    pub signup_daily_ip_window: Duration,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            login_ip_limit: 10,
            login_ip_window: Duration::from_secs(60),
            login_account_limit: 5,
            login_account_window: Duration::from_secs(300),
            login_ip_account_limit: 5,
            login_ip_account_window: Duration::from_secs(300),
            signup_ip_limit: 5,
            signup_ip_window: Duration::from_secs(3600),
            signup_email_limit: 3,
            signup_email_window: Duration::from_secs(3600),
            signup_daily_ip_limit: 20,
            signup_daily_ip_window: Duration::from_secs(86400),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TrustedProxyNetwork {
    address: IpAddr,
    prefix: u8,
}

impl TrustedProxyNetwork {
    pub fn contains(&self, address: IpAddr) -> bool {
        match (self.address, address) {
            (IpAddr::V4(network), IpAddr::V4(address)) => {
                let mask = if self.prefix == 0 { 0 } else { u32::MAX << (32 - self.prefix) };
                u32::from(network) & mask == u32::from(address) & mask
            }
            (IpAddr::V6(network), IpAddr::V6(address)) => {
                let mask = if self.prefix == 0 { 0 } else { u128::MAX << (128 - self.prefix) };
                u128::from(network) & mask == u128::from(address) & mask
            }
            _ => false,
        }
    }
}

impl FromStr for TrustedProxyNetwork {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let (address, prefix) = match value.split_once('/') {
            Some((address, prefix)) => {
                let address = address.parse::<IpAddr>().map_err(|_| format!("invalid proxy IP network '{value}'"))?;
                let prefix = prefix.parse::<u8>().map_err(|_| format!("invalid proxy network prefix '{value}'"))?;
                (address, prefix)
            }
            None => {
                let address = value.parse::<IpAddr>().map_err(|_| format!("invalid proxy IP network '{value}'"))?;
                let prefix = if address.is_ipv4() { 32 } else { 128 };
                (address, prefix)
            }
        };

        let max_prefix = if address.is_ipv4() { 32 } else { 128 };
        if prefix > max_prefix {
            return Err(format!("proxy network prefix is out of range in '{value}'"));
        }

        Ok(Self { address, prefix })
    }
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
        Self { secure: !environment.is_development(), http_only: true, same_site: CookieSameSite::Lax }
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

fn positive_u64(name: &str, default: u64, errors: &mut Vec<String>) -> u64 {
    match env::var(name) {
        Ok(value) => match value.parse::<u64>() {
            Ok(value) if value > 0 => value,
            _ => {
                errors.push(format!("{name}: expected a positive integer"));
                default
            }
        },
        Err(env::VarError::NotPresent) => default,
        Err(env::VarError::NotUnicode(_)) => {
            errors.push(format!("{name}: value is not valid Unicode"));
            default
        }
    }
}

fn positive_usize(name: &str, default: usize, errors: &mut Vec<String>) -> usize {
    match env::var(name) {
        Ok(value) => match value.parse::<usize>() {
            Ok(value) if value > 0 => value,
            _ => {
                errors.push(format!("{name}: expected a positive integer"));
                default
            }
        },
        Err(env::VarError::NotPresent) => default,
        Err(env::VarError::NotUnicode(_)) => {
            errors.push(format!("{name}: value is not valid Unicode"));
            default
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

    #[test]
    fn trusted_proxy_network_matches_only_addresses_in_its_prefix() {
        let network = "192.0.2.0/24".parse::<TrustedProxyNetwork>().unwrap();

        assert!(network.contains("192.0.2.42".parse().unwrap()));
        assert!(!network.contains("192.0.3.42".parse().unwrap()));
        assert!(!network.contains("2001:db8::1".parse().unwrap()));
    }

    #[test]
    fn trusted_proxy_network_supports_ipv6_prefixes() {
        let network = "2001:db8:abcd::/48".parse::<TrustedProxyNetwork>().unwrap();

        assert!(network.contains("2001:db8:abcd::1".parse().unwrap()));
        assert!(!network.contains("2001:db8:abce::1".parse().unwrap()));
    }
}
