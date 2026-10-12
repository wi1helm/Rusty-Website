use axum::{
    body::Body,
    extract::{Path, State},
    http::{header, HeaderValue, StatusCode},
    response::Response,
};

use crate::{
    storage::object::ObjectStorageError,
    Context,
};

pub async fn get_image(State(context): State<Context>, Path(key): Path<String>) -> Result<Response, StatusCode> {
    // We only serve WebP images.
    if !key.ends_with(".webp") { return Err(StatusCode::NOT_FOUND); }

    let bytes = match context.public_object_storage.get(&key).await {
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