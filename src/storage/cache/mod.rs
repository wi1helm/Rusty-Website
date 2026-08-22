use async_trait::async_trait;
use std::time::Duration;
pub(crate) mod redis;

#[derive(Debug, thiserror::Error)]
pub enum CacheError {
    #[error("redis error")]
    Redis(#[from] ::redis::RedisError),
}

#[async_trait]
pub trait Cache: Send + Sync {
    async fn get(&self, key: &str) -> Result<Option<Vec<u8>>, CacheError>;

    async fn set(&self, key: &str, value: &[u8], ttl: Duration) -> Result<(), CacheError>;

    async fn delete(&self, key: &str) -> Result<(), CacheError>;
}
