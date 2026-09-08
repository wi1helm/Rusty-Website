pub(crate) mod s3;

use async_trait::async_trait;

#[derive(Debug)]
pub enum ObjectStorageError {
    NotFound,
    Storage(String),
}
#[async_trait]
pub trait ObjectStorage: Send + Sync {
    async fn put(&self, key: &str, data: Vec<u8>, content_type: &str) -> Result<(), ObjectStorageError>;

    async fn get(&self, key: &str) -> Result<Vec<u8>, ObjectStorageError>;

    async fn delete(&self, key: &str) -> Result<(), ObjectStorageError>;

    fn url(&self, key: &str) -> String;
}

