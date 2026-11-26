use axum::{Json, http::{StatusCode, header}, response::{IntoResponse, Response}};
use serde::Serialize;

#[derive(Serialize)]
pub struct ApiErrorResponse {
    pub error: ApiErrorDetail,
}

#[derive(Serialize)]
pub struct ApiErrorDetail {
    pub code: &'static str,
    pub message: &'static str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiSuccessResponse {
    pub redirect_to: &'static str,
}

impl ApiSuccessResponse {
    pub fn redirect_to(redirect_to: &'static str) -> Self {
        Self { redirect_to }
    }
}

pub fn api_error(status: StatusCode, code: &'static str, message: &'static str) -> Response {
    (
        status,
        Json(ApiErrorResponse {
            error: ApiErrorDetail { code, message },
        }),
    ).into_response()
}

pub async fn normalize_error_response(response: Response) -> Response {
    if !response.status().is_client_error() && !response.status().is_server_error() { return response; }

    if response.headers()
        .get(header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|content_type| content_type.starts_with("application/json"))
    { return response; }

    let status = response.status();
    let headers = response.headers().clone();
    let (code, message) = match status {
        StatusCode::BAD_REQUEST | StatusCode::UNPROCESSABLE_ENTITY => ("invalid-request", "The request could not be processed. Check the submitted information and try again.",),
        StatusCode::UNAUTHORIZED => ("authentication-required", "Please sign in to continue.",),
        StatusCode::FORBIDDEN => ("forbidden", "You are not allowed to perform this action."),
        StatusCode::NOT_FOUND => ("not-found", "The requested resource was not found."),
        StatusCode::CONFLICT => ("conflict", "The request conflicts with the current resource."),
        StatusCode::PAYLOAD_TOO_LARGE => ("payload-too-large", "The uploaded content is larger than the allowed limit.",),
        StatusCode::UNSUPPORTED_MEDIA_TYPE => ("unsupported-media-type", "The submitted content type is not supported.",),
        StatusCode::SERVICE_UNAVAILABLE => ("service-unavailable", "The service is temporarily unavailable. Please try again.",),
        _ if status.is_server_error() => ("server-error", "Something went wrong. Please try again.",),
        _ => ("request-failed", "The request could not be completed."),
    };

    let mut normalized = api_error(status, code, message);
    for name in [
        header::ALLOW,
        header::RETRY_AFTER,
        header::SET_COOKIE,
        header::WWW_AUTHENTICATE,
    ] {
        for value in headers.get_all(&name) {
            normalized.headers_mut().append(name.clone(), value.clone());
        }
    }
    normalized
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn normalizes_status_only_errors_to_structured_json() {
        let response = normalize_error_response(StatusCode::BAD_REQUEST.into_response()).await;

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        assert_eq!(response.headers()[header::CONTENT_TYPE], "application/json");
        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let body: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(body["error"]["code"], "invalid-request");
        assert!(body["error"]["message"].as_str().is_some());
    }
}