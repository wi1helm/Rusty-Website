// database/mod.rs
pub mod postgres;
pub use postgres::PostgresDatabase;

use async_trait::async_trait;

#[async_trait]
pub trait Database: Send + Sync {
    // add more methods as you need them
}