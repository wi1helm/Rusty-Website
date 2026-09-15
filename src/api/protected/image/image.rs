use askama_web::__askama_web_impl::axum_core_0_5::IntoResponse;
use axum::{
    extract::{Multipart, State},
    extract::multipart::Field,
    http::StatusCode,
    response::Redirect,
};

use crate::{
    auth::CurrentUser,
    storage::database::ProfileImageType,
    Context,
};

use super::validator::ImageValidator;
use super::MAX_IMAGE_SIZE;

const MAX_LENGTH_OF_IMAGE_TYPE: usize = 32;

struct UploadedImage {
    image_type: ProfileImageType,
    data: Vec<u8>,
    content_type: String,
}

pub async fn upload(State(context): State<Context>, CurrentUser(user): CurrentUser, multipart: Multipart) -> impl IntoResponse {
    let uploaded = match parse_multipart(multipart, user.organization.id).await {
        Ok(uploaded) => uploaded,
        Err(status) => return status.into_response()
    };

    let validated = match ImageValidator::validate(uploaded.image_type, uploaded.data, &uploaded.content_type) {
        Ok(image) => image,
        Err(error) => {
            tracing::debug!(?error, organization_id = user.organization.id, "profile image failed validation");
            return error.status_code().into_response();
        }
    };

    tracing::debug!(
        organization_id = user.organization.id,
        image_type = validated.image_type.as_str(),
        content_type = validated.content_type,
        width = validated.width,
        height = validated.height,
        file_size = validated.data.len(),
        "profile image passed validation"
    );

    // TODO:
    // Generate storage key.

    // TODO:
    // Upload to object storage.

    // TODO:
    // Save image to database.

    // TODO:
    // Delete old object after successful database update.

    let _ = context;

    Redirect::to("/profile").into_response()
}

async fn parse_multipart(mut multipart: Multipart, organization_id: i64) -> Result<UploadedImage, StatusCode> {
    let mut image_type = None;
    let mut image_data = None;
    let mut content_type = None;

    while let Some(mut field) = next_field(&mut multipart, organization_id).await? {
        match field.name() {
            Some("image_type") => {
                parse_image_type(field, &mut image_type, organization_id).await?;
            }

            Some("image") => {
                parse_image(&mut field, &mut image_data, &mut content_type, organization_id).await?;
            }

            Some(name) => {
                tracing::debug!(organization_id, field = name, "unexpected multipart field");
                return Err(StatusCode::BAD_REQUEST);
            }

            None => {
                tracing::debug!(organization_id, "multipart field without a name");
                return Err(StatusCode::BAD_REQUEST);
            }
        }
    }

    let image_type = image_type.ok_or(StatusCode::BAD_REQUEST)?;
    let image_data = image_data.ok_or(StatusCode::BAD_REQUEST)?;
    let content_type = content_type.ok_or(StatusCode::BAD_REQUEST)?;

    Ok(UploadedImage { image_type, data: image_data, content_type})
}

/// Reads the next multipart field and converts multipart parsing errors into
/// a `400 Bad Request` response.
async fn next_field(multipart: &mut Multipart, organization_id: i64) -> Result<Option<Field<'_>>, StatusCode> {
    multipart.next_field().await.map_err(|error| {
        tracing::warn!(?error, organization_id, "failed to read multipart field");
        StatusCode::BAD_REQUEST
    })
}

/// Reads and parses the `image_type` multipart field.
///
/// The field is consumed because `Field::text()` takes ownership of the field.
/// The value is bounded before it is converted into a `ProfileImageType`.
async fn parse_image_type(field: Field<'_>, image_type: &mut Option<ProfileImageType>, organization_id: i64) -> Result<(), StatusCode> {
    if image_type.is_some() {
        tracing::debug!(organization_id, "duplicate image_type field");
        return Err(StatusCode::BAD_REQUEST);
    }

    let value = field.text().await.map_err(|error| {
        tracing::warn!(?error, organization_id, "failed to read image type");
        StatusCode::BAD_REQUEST
    })?;

    if value.len() > MAX_LENGTH_OF_IMAGE_TYPE { return Err(StatusCode::BAD_REQUEST); }

    *image_type = ProfileImageType::from_str(value.trim());

    Ok(())
}

/// Reads the image multipart field and stores its content type and bytes.
///
/// The image data is streamed in chunks so the upload can be rejected as soon
/// as it exceeds `MAX_IMAGE_SIZE`.
async fn parse_image(field: &mut Field<'_>, image_data: &mut Option<Vec<u8>>, content_type: &mut Option<String>, organization_id: i64) -> Result<(), StatusCode> {
    if image_data.is_some() {
        tracing::debug!(organization_id, "duplicate image field");
        return Err(StatusCode::BAD_REQUEST);
    }

    let field_content_type = field
        .content_type()
        .ok_or(StatusCode::BAD_REQUEST)?
        .to_owned();

    let data = read_image_data(field, organization_id).await?;

    *image_data = Some(data);
    *content_type = Some(field_content_type);

    Ok(())
}

/// Reads an image field incrementally while enforcing the maximum upload size.
///
/// This deliberately avoids `Field::bytes()`, which would read the entire
/// field before we can enforce the image-specific size limit.
async fn read_image_data(field: &mut Field<'_>, organization_id: i64, ) -> Result<Vec<u8>, StatusCode> {
    let mut data = Vec::new();

    while let Some(chunk) = field.chunk().await
        .map_err(|error| {
            tracing::warn!(?error, organization_id, "failed to read uploaded image");
            StatusCode::BAD_REQUEST
        })?
    {
        if data.len() + chunk.len() > MAX_IMAGE_SIZE {
            tracing::debug!(organization_id, "uploaded image exceeded maximum size");
            return Err(StatusCode::PAYLOAD_TOO_LARGE);
        }

        data.extend_from_slice(&chunk);
    }

    Ok(data)
}

#[cfg(test)]
mod tests {
    use super::*;

    use axum::{body::Body, extract::FromRequest, http::{header, Request}, Router};
    use axum::extract::DefaultBodyLimit;
    use crate::api::protected::image::MAX_REQUEST_SIZE;
    use tower::ServiceExt;
    fn multipart_body(boundary: &str, fields: &[(&str, &str, &[u8], Option<&str>)], ) -> Vec<u8> {
        let mut body = Vec::new();

        for (name, filename, data, content_type) in fields {
            body.extend_from_slice(format!("--{boundary}\r\n").as_bytes());

            body.extend_from_slice(format!("Content-Disposition: form-data; name=\"{name}\"; filename=\"{filename}\"\r\n").as_bytes());

            if let Some(content_type) = content_type {
                body.extend_from_slice(format!("Content-Type: {content_type}\r\n").as_bytes());
            }
            body.extend_from_slice(b"\r\n");
            body.extend_from_slice(data);
            body.extend_from_slice(b"\r\n");
        }
        body.extend_from_slice(format!("--{boundary}--\r\n").as_bytes());
        body
    }

    async fn parse_test_multipart(body: Vec<u8>, boundary: &str) -> Result<UploadedImage, StatusCode> {
        // Shared state slot to capture the uploaded image on success
        let captured_image = std::sync::Arc::new(tokio::sync::Mutex::new(None));
        let captured_clone = captured_image.clone();

        let app = Router::new()
            .route("/",
                axum::routing::post(move |multipart: Multipart| async move {
                    match parse_multipart(multipart, 123).await {
                        Ok(uploaded) => {
                            *captured_clone.lock().await = Some(uploaded);
                            Ok(StatusCode::OK)
                        }
                        Err(status) => Err(status),
                    }
                }),
            )
            .layer(DefaultBodyLimit::max(MAX_REQUEST_SIZE));

        let request = Request::builder()
            .method("POST")
            .uri("/")
            .header(header::CONTENT_TYPE, format!("multipart/form-data; boundary={boundary}"))
            .body(Body::from(body))
            .unwrap();

        let response = app.oneshot(request).await.unwrap();

        if response.status().is_success() {
            let uploaded = captured_image
                .lock()
                .await
                .take()
                .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
            Ok(uploaded)
        } else {
            Err(response.status())
        }
    }

    #[tokio::test]
    async fn accepts_valid_multipart_upload() {
        let image = b"fake image data";

        let body = multipart_body(
            "test-boundary",
            &[
                ("image_type", "", b"logo", None),
                ("image", "logo.png", image, Some("image/png")),
            ],
        );

        let result = parse_test_multipart(body, "test-boundary").await;

        let uploaded = result.unwrap();

        assert!(matches!(uploaded.image_type, ProfileImageType::Logo));
        assert_eq!(uploaded.data, image);
        assert_eq!(uploaded.content_type, "image/png");
    }

    #[tokio::test]
    async fn rejects_missing_image_type() {
        let body = multipart_body(
            "test-boundary",
            &[
                ("image", "logo.png", b"fake image data", Some("image/png")),
            ],
        );

        let result = parse_test_multipart(body, "test-boundary").await;

        assert!(matches!(result, Err(StatusCode::BAD_REQUEST)));
    }

    #[tokio::test]
    async fn rejects_missing_image() {
        let body = multipart_body(
            "test-boundary",
            &[
                ("image_type", "", b"logo", None),
            ],
        );

        let result = parse_test_multipart(body, "test-boundary").await;

        assert!(matches!(result, Err(StatusCode::BAD_REQUEST)));
    }

    #[tokio::test]
    async fn rejects_duplicate_image_type() {
        let body = multipart_body(
            "test-boundary",
            &[
                ("image_type", "", b"logo", None),
                ("image_type", "", b"banner", None),
                ("image", "logo.png", b"fake image data", Some("image/png")),
            ],
        );

        let result = parse_test_multipart(body, "test-boundary").await;

        assert!(matches!(result, Err(StatusCode::BAD_REQUEST)));
    }

    #[tokio::test]
    async fn rejects_duplicate_image() {
        let body = multipart_body(
            "test-boundary",
            &[
                ("image_type", "", b"logo", None),
                ("image", "logo.png", b"first image", Some("image/png")),
                ("image", "logo2.png", b"second image", Some("image/png")),
            ],
        );

        let result = parse_test_multipart(body, "test-boundary").await;

        assert!(matches!(result, Err(StatusCode::BAD_REQUEST)));
    }

    #[tokio::test]
    async fn rejects_unknown_field() {
        let body = multipart_body(
            "test-boundary",
            &[
                ("image_type", "", b"logo", None),
                ("image", "logo.png", b"fake image data", Some("image/png")),
                ("something_else", "", b"unexpected", None),
            ],
        );

        let result = parse_test_multipart(body, "test-boundary").await;

        assert!(matches!(result, Err(StatusCode::BAD_REQUEST)));
    }

    #[tokio::test]
    async fn rejects_image_type_that_is_too_long() {
        let image_type = "a".repeat(MAX_LENGTH_OF_IMAGE_TYPE + 1);

        let body = multipart_body(
            "test-boundary",
            &[
                ("image_type", "", image_type.as_bytes(), None),
                ("image", "logo.png", b"fake image data", Some("image/png")),
            ],
        );

        let result = parse_test_multipart(body, "test-boundary").await;

        assert!(matches!(result, Err(StatusCode::BAD_REQUEST)));
    }

    #[tokio::test]
    async fn rejects_invalid_image_type() {
        let body = multipart_body(
            "test-boundary",
            &[
                ("image_type", "", b"something_else", None),
                ("image", "logo.png", b"fake image data", Some("image/png")),
            ],
        );

        let result = parse_test_multipart(body, "test-boundary").await;

        assert!(matches!(result, Err(StatusCode::BAD_REQUEST)));
    }

    #[tokio::test]
    async fn rejects_missing_image_content_type() {
        let body = multipart_body(
            "test-boundary",
            &[
                ("image_type", "", b"logo", None),
                ("image", "logo.png", b"fake image data", None),
            ],
        );

        let result = parse_test_multipart(body, "test-boundary").await;

        assert!(matches!(result, Err(StatusCode::BAD_REQUEST)));
    }

    #[tokio::test]
    async fn rejects_image_larger_than_maximum_size() {
        let image = vec![0u8; MAX_IMAGE_SIZE + 1];

        let body = multipart_body(
            "test-boundary",
            &[
                ("image_type", "", b"logo", None),
                ("image", "large.png", &image, Some("image/png")),
            ],
        );

        let result = parse_test_multipart(body, "test-boundary").await;

        assert!(matches!(result, Err(StatusCode::PAYLOAD_TOO_LARGE)));
    }

    #[tokio::test]
    async fn accepts_image_at_maximum_size() {
        let image = vec![0u8; MAX_IMAGE_SIZE];

        let body = multipart_body(
            "test-boundary",
            &[
                ("image_type", "", b"logo", None),
                ("image", "large.png", &image, Some("image/png")),
            ],
        );

        let result = parse_test_multipart(body, "test-boundary").await;

        assert!(result.is_ok());
    }
}