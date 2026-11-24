use axum::{
    body::Body,
    extract::{Path, State},
    http::{HeaderValue, StatusCode, header},
    response::Response,
};
use std::sync::Arc;

use crate::storage::object::{ObjectStorage, ObjectStorageError};

pub async fn get_image(State(object_storage): State<Arc<dyn ObjectStorage>>, Path(key): Path<String>) -> Result<Response, StatusCode> {
    // We only serve WebP images.
    if !key.ends_with(".webp") { return Err(StatusCode::NOT_FOUND); }

    let bytes = match object_storage.get(&key).await {
        Ok(bytes) => bytes,

        Err(ObjectStorageError::NotFound) => { return Err(StatusCode::NOT_FOUND); }
        Err(ObjectStorageError::Storage(error)) => {
            tracing::error!(image_key = %key, error = %error, "Failed to retrieve image from object storage");
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    let mut response = Response::new(Body::from(bytes));

    *response.status_mut() = StatusCode::OK;

    response.headers_mut().insert(header::CONTENT_TYPE, HeaderValue::from_static("image/webp"));
    response.headers_mut().insert(header::CACHE_CONTROL, HeaderValue::from_static("public, max-age=3600"));

    Ok(response)
}
