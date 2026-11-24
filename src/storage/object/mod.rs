pub mod defaults;
pub mod s3;

use async_trait::async_trait;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ObjectStorageError {
    #[error("Object not found")]
    NotFound,
    #[error("Storage error: {0}")]
    Storage(String),
}
#[async_trait]
pub trait ObjectStorage: Send + Sync {
    async fn put(&self, key: &str, data: Vec<u8>, content_type: &str) -> Result<(), ObjectStorageError>;

    async fn get(&self, key: &str) -> Result<Vec<u8>, ObjectStorageError>;

    async fn delete(&self, key: &str) -> Result<(), ObjectStorageError>;

    fn url(&self, key: &str) -> String;
}
