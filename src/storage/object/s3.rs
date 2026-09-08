use async_trait::async_trait;
use aws_sdk_s3::{
    config::{Credentials, Region},
    primitives::ByteStream,
    Client,
};

use super::{ObjectStorage, ObjectStorageError};

/// S3-compatible object storage implementation.
///
/// This works with MinIO locally and can also work with
/// Amazon S3, Cloudflare R2, or other S3-compatible providers.
pub struct S3ObjectStorage {
    /// AWS S3 SDK client used to communicate with the storage server.
    client: Client,

    /// Name of the bucket where our objects are stored.
    bucket: String,

    /// Base URL that browsers can use to access stored objects.
    ///
    /// This is separate from the S3 endpoint because the endpoint
    /// is used by the application, while this URL is used by clients.
    pub public_url: String,
}

impl S3ObjectStorage {
    /// Creates a new S3 object storage instance.
    ///
    /// The endpoint can point to MinIO locally or an external
    /// S3-compatible provider in production.
    pub async fn new(endpoint: &str, bucket: String, access_key: &str, secret_key: &str, public_url: String) -> Result<Self, ObjectStorageError> {
        //TODO SECURITY :
        // Use a dedicated least-privilege object-storage identity for the
        // application instead of MinIO root/admin credentials.
        // The application should only have the permissions it actually needs
        // for the application bucket (e.g. PutObject, GetObject, DeleteObject).
        // Administrative operations such as creating buckets, users, or
        // changing policies should remain restricted to the MinIO administrator.


        // Create credentials for authenticating with the S3 server.
        //
        // For local development these will be the MinIO credentials.
        // In production these would be your actual storage credentials.
        let credentials = Credentials::new(
            access_key,
            secret_key,
            None,
            None,
            "app",
        );

        // Configure the S3 client.
        let config = aws_sdk_s3::Config::builder()
            // Where the S3-compatible server is located.
            //
            // Locally this could be:
            // http://minio:9000
            //
            // With Amazon S3 this would be an AWS S3 endpoint.
            .endpoint_url(endpoint)

            // Tell the SDK which credentials to use.
            .credentials_provider(credentials)

            // S3 requires a region.
            //
            // MinIO doesn't really care about the region, so this
            // is mainly here to satisfy the S3 client's configuration.
            .region(Region::new("eu-north-1"))

            // Use path-style URLs.
            //
            // This is important for MinIO because it allows requests
            // to look like:
            // http://minio:9000/my-bucket/my-file.webp
            //
            // instead of:
            // http://my-bucket.minio:9000/my-file.webp
            .force_path_style(true)

            .build();

        // Create the S3 client from our configuration.
        let client = Client::from_conf(config);

        // Create our storage object.
        let storage = Self {
            client,
            bucket,
            public_url,
        };

        // Check whether the configured bucket already exists.
        //
        // This means the application doesn't require us to manually
        // create the bucket when starting MinIO for the first time.
        match storage.client.head_bucket().bucket(&storage.bucket).send().await {
            // The bucket already exists, so we're ready to use it.
            Ok(_) => {
                // Nothing to do.
            }
            // Something went wrong checking the bucket.
            //
            // For now we assume this means the bucket doesn't exist
            // and attempt to create it.
            Err(_) => {
                storage
                .client
                .create_bucket()
                .bucket(&storage.bucket)
                .send()
                .await
                .map_err(|error| {
                    ObjectStorageError::Storage(error.to_string())
                })?;
            }
        }

        Ok(storage)
    }
}

#[async_trait]
impl ObjectStorage for S3ObjectStorage {
    /// Upload an object to the configured bucket.
    async fn put(&self, key: &str, data: Vec<u8>, content_type: &str, ) -> Result<(), ObjectStorageError> {
        self.client
            .put_object()
            .bucket(&self.bucket)

            // The key is the object's path/name.
            //
            // Example:
            // organizations/123/logo/abc.webp
            .key(key)

            // Tell the storage server what type of file this is.
            //
            // Example:
            // image/webp
            .content_type(content_type)

            // Convert our Vec<u8> into a stream that the S3 SDK can upload.
            .body(ByteStream::from(data))

            // Actually send the request to the S3-compatible server.
            .send()
            .await
            .map_err(|error| {
                ObjectStorageError::Storage(error.to_string())
            })?;

        Ok(())
    }

    /// Download an object from the configured bucket.
    async fn get(&self, key: &str) -> Result<Vec<u8>, ObjectStorageError> {
        // Request the object from S3/MinIO.
        let result = self
            .client
            .get_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await
            .map_err(|error| {
                ObjectStorageError::Storage(error.to_string())
            })?;

        // The response body is a stream.
        //
        // Collect the entire stream into memory.
        let data = result
            .body
            .collect()
            .await
            .map_err(|error| {
                ObjectStorageError::Storage(error.to_string())
            })?;

        // Convert the collected data into Vec<u8>.
        Ok(data.into_bytes().to_vec())
    }

    /// Delete an object from the configured bucket.
    async fn delete(&self, key: &str) -> Result<(), ObjectStorageError> {
        self.client
            .delete_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await
            .map_err(|error| {
                ObjectStorageError::Storage(error.to_string())
            })?;

        Ok(())
    }

    /// Generate the public URL for an object.
    ///
    /// For example:
    ///
    /// public_url:
    /// http://localhost:9000/my-app
    ///
    /// key:
    /// organizations/123/logo/abc.webp
    ///
    /// Result:
    /// http://localhost:9000/my-app/organizations/123/logo/abc.webp
    fn url(&self, key: &str) -> String {
        format!("{}/{}", self.public_url, key)
    }
}