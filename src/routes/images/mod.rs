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
    if !key.ends_with(".webp") {
        return Err(StatusCode::NOT_FOUND);
    }

    let bytes = match object_storage.get(&key).await {
        Ok(bytes) => bytes,

        Err(ObjectStorageError::NotFound) => {
            return Err(StatusCode::NOT_FOUND);
        }
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

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        Router,
        body::{Body, to_bytes},
        http::{Request, header},
        routing::get,
    };
    use std::sync::atomic::{AtomicUsize, Ordering};
    use tower::ServiceExt;

    #[derive(Clone, Copy)]
    enum GetOutcome {
        Found,
        Missing,
        Failed,
    }

    struct FakeObjectStorage {
        outcome: GetOutcome,
        get_calls: Arc<AtomicUsize>,
    }

    #[async_trait::async_trait]
    impl ObjectStorage for FakeObjectStorage {
        async fn put(&self, _key: &str, _data: Vec<u8>, _content_type: &str) -> Result<(), ObjectStorageError> {
            Ok(())
        }

        async fn get(&self, _key: &str) -> Result<Vec<u8>, ObjectStorageError> {
            self.get_calls.fetch_add(1, Ordering::SeqCst);
            match self.outcome {
                GetOutcome::Found => Ok(b"image bytes".to_vec()),
                GetOutcome::Missing => Err(ObjectStorageError::NotFound),
                GetOutcome::Failed => Err(ObjectStorageError::Storage("unavailable".to_owned())),
            }
        }

        async fn delete(&self, _key: &str) -> Result<(), ObjectStorageError> {
            Ok(())
        }

        fn url(&self, key: &str) -> String {
            format!("https://images.example/{key}")
        }
    }

    fn image_router(outcome: GetOutcome) -> (Router, Arc<AtomicUsize>) {
        let get_calls = Arc::new(AtomicUsize::new(0));
        let storage: Arc<dyn ObjectStorage> = Arc::new(FakeObjectStorage { outcome, get_calls: get_calls.clone() });
        let router = Router::new().route("/images/{*key}", get(get_image)).with_state(storage);

        (router, get_calls)
    }

    async fn get_image_response(router: Router, path: &str) -> Response {
        router.oneshot(Request::builder().uri(path).body(Body::empty()).unwrap()).await.unwrap()
    }

    #[tokio::test]
    async fn image_route_returns_webp_with_cache_headers() {
        let (router, get_calls) = image_router(GetOutcome::Found);
        let response = get_image_response(router, "/images/logo.webp").await;

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.headers()[header::CONTENT_TYPE], "image/webp");
        assert_eq!(response.headers()[header::CACHE_CONTROL], "public, max-age=3600");
        assert_eq!(to_bytes(response.into_body(), 1024).await.unwrap(), "image bytes");
        assert_eq!(get_calls.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn image_route_rejects_non_webp_without_reading_storage() {
        let (router, get_calls) = image_router(GetOutcome::Found);
        let response = get_image_response(router, "/images/logo.png").await;

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        assert_eq!(get_calls.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn image_route_maps_storage_errors_to_http_statuses() {
        for (outcome, expected_status) in [(GetOutcome::Missing, StatusCode::NOT_FOUND), (GetOutcome::Failed, StatusCode::INTERNAL_SERVER_ERROR)] {
            let (router, get_calls) = image_router(outcome);
            let response = get_image_response(router, "/images/logo.webp").await;

            assert_eq!(response.status(), expected_status);
            assert_eq!(get_calls.load(Ordering::SeqCst), 1);
        }
    }
}
