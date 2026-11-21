//! Application configuration.
//!
//! TODO: Introduce a proper `Config` struct that validates all required
//! environment variables at once with meaningful error messages.
//! Currently all real configuration (DATABASE_URL, S3 credentials, etc.)
//! is scattered as raw `std::env::var()` calls throughout `main()`.
//! A Config struct would centralize configuration, make local development
//! easier, and fail fast with a single error message listing all missing
//! variables instead of panicking on the first one.
//! Consider using `dotenvy` for .env file support or `config-rs` crate
//! for multi-source configuration (env + files + defaults).
//! Search terms: "Rust configuration management", "config-rs crate",
//! "dotenvy environment variables Rust", "validated config pattern".

pub const APP_NAME: &str = "Rust App 4000";
pub const APP_YEAR: u16 = 2026;
pub const DEFAULT_LOGO_KEY: &str = "defaults/default_logo.webp";
pub const DEFAULT_BANNER_KEY: &str = "defaults/default_banner.webp";
