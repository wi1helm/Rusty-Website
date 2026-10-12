use async_trait::async_trait;
use aws_sdk_s3::{
    config::{Credentials, Region},
    primitives::ByteStream,
    Client,
};
use aws_sdk_s3::error::ProvideErrorMetadata;
use crate::config::{DEFAULT_BANNER_KEY, DEFAULT_LOGO_KEY};

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
        //TODO SECURITY:
        // Use a dedicated least-privilege object-storage identity for the
        // application instead of MinIO root/admin credentials.
        // The application should only have the permissions it actually
        // needs for the application bucket.
        // Administrative operations such as creating buckets, users,
        // or changing policies should eventually be handled by a
        // dedicated infrastructure/deployment identity.

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
            // Locally:
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
            //
            // http://minio:9000/my-bucket/my-file.webp
            //
            // instead of:
            //
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

        // Make sure the bucket exists.
        storage.ensure_bucket().await?;

        // Profile images are public content, so allow browsers
        // to download objects from this bucket without authentication.
        //TODO removes due to images going trough rust application and not direcyly to database. could be bad for bandwith etc. check later on.
        //storage.ensure_public_access().await?;

        // Make sure the default profile images exist.
        storage.ensure_default_objects().await?;

        Ok(storage)
    }

    /// Makes sure the configured bucket exists.
    async fn ensure_bucket(&self) -> Result<(), ObjectStorageError> {
        match self.client.head_bucket().bucket(&self.bucket).send().await {
            Ok(_) => {
                // The bucket already exists.
                Ok(())
            }

            Err(_) => {
                // The bucket does not appear to exist, so create it.
                //
                // TODO:
                // Eventually distinguish "bucket does not exist" from
                // authentication/network/permission errors.
                self.client
                    .create_bucket()
                    .bucket(&self.bucket)
                    .send()
                    .await
                    .map_err(|error| {
                        ObjectStorageError::Storage(error.to_string())
                    })?;

                Ok(())
            }
        }
    }

    /// Configures the bucket so its objects can be downloaded publicly.
    ///
    /// This is needed because profile logos and banners are public
    /// profile content and must be accessible by browsers without
    /// S3 credentials.
    async fn ensure_public_access(&self) -> Result<(), ObjectStorageError> {
        let policy = format!(
            r#"{{
                "Version": "2012-10-17",
                "Statement": [
                    {{
                        "Effect": "Allow",
                        "Principal": "*",
                        "Action": "s3:GetObject",
                        "Resource": "arn:aws:s3:::{bucket}/*"
                    }}
                ]
            }}"#,
            bucket = self.bucket,
        );

        self.client
            .put_bucket_policy()
            .bucket(&self.bucket)
            .policy(policy)
            .send()
            .await
            .map_err(|error| {
                ObjectStorageError::Storage(error.to_string())
            })?;

        Ok(())
    }

    /// Makes sure the application's default images exist.
    async fn ensure_default_objects(&self) -> Result<(), ObjectStorageError> {
        self.ensure_default_object(
            DEFAULT_LOGO_KEY,
            include_bytes!("../../../static/defaults/default_logo.webp"),
            "image/webp",
        )
            .await?;

        self.ensure_default_object(
            DEFAULT_BANNER_KEY,
            include_bytes!("../../../static/defaults/default_banner.webp"),
            "image/webp",
        )
            .await?;

        Ok(())
    }

    /// Makes sure a specific default object exists.
    async fn ensure_default_object(&self, key: &str, data: &[u8], content_type: &str, ) -> Result<(), ObjectStorageError> {
        match self
            .client
            .head_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await
        {
            Ok(_) => {
                // Already exists.
                Ok(())
            }

            Err(_) => {
                // Doesn't exist, so create it.
                self.client
                    .put_object()
                    .bucket(&self.bucket)
                    .key(key)
                    .content_type(content_type)
                    .body(ByteStream::from(data.to_vec()))
                    .send()
                    .await
                    .map_err(|error| {
                        ObjectStorageError::Storage(error.to_string())
                    })?;

                Ok(())
            }
        }
    }
}

#[async_trait]
impl ObjectStorage for S3ObjectStorage {
    /// Upload an object to the configured bucket.
    async fn put(&self, key: &str, data: Vec<u8>, content_type: &str) -> Result<(), ObjectStorageError> {
        self.client
            .put_object()
            .bucket(&self.bucket)
            .key(key)
            .content_type(content_type)
            .body(ByteStream::from(data))
            .send()
            .await
            .map_err(|error| {
                ObjectStorageError::Storage(error.to_string())
            })?;

        Ok(())
    }

    /// Download an object from the configured bucket.
    async fn get(&self, key: &str) -> Result<Vec<u8>, ObjectStorageError> {
        let result = self
            .client
            .get_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await
            .map_err(|error| {
                // Distinguish a missing object from other storage failures.
                if error.as_service_error().and_then(|service_error| service_error.code()) == Some("NoSuchKey") {
                    ObjectStorageError::NotFound
                } else {
                    ObjectStorageError::Storage(error.to_string())
                }
            })?;

        let data = result
            .body
            .collect()
            .await
            .map_err(|error| ObjectStorageError::Storage(error.to_string()))?;

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
    /// http://localhost:9000/app
    ///
    /// key:
    /// defaults/default_banner.webp
    ///
    /// Result:
    /// http://localhost:9000/app/defaults/default_banner.webp
    fn url(&self, key: &str) -> String {
        format!("{}/{}", self.public_url, key)
    }
}