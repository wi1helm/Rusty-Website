use askama::Template;
use askama_web::WebTemplate;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};

use crate::config::{APP_NAME, APP_YEAR};

#[derive(Template, WebTemplate)]
#[template(path = "errors/http_error.html")]
struct HttpErrorTemplate {
    app_name: &'static str,
    year: u16,
    status_code: u16,
    title: &'static str,
    message: &'static str,
}

pub fn get_page(status: StatusCode) -> Response {
    let (title, message) = match status {
        StatusCode::BAD_REQUEST => (
            "We couldn't process that request",
            "Please check the information and try again.",
        ),
        StatusCode::UNAUTHORIZED => ("Please sign in", "Sign in to continue to that page."),
        StatusCode::FORBIDDEN => (
            "Request blocked",
            "You can't perform that action from this page. Please return and try again.",
        ),
        StatusCode::NOT_FOUND => (
            "Page not found",
            "Sorry, we couldn't find the page you're looking for.",
        ),
        StatusCode::METHOD_NOT_ALLOWED => {
            ("Action not available", "That action isn't available here.")
        }
        StatusCode::CONFLICT => (
            "This conflicts with existing information",
            "Please review the information and try again.",
        ),
        StatusCode::PAYLOAD_TOO_LARGE => (
            "That file is too large",
            "Choose a smaller file and try again.",
        ),
        StatusCode::UNSUPPORTED_MEDIA_TYPE => (
            "Unsupported format",
            "Please use a supported format and try again.",
        ),
        StatusCode::UNPROCESSABLE_ENTITY => (
            "Please check your information",
            "Some of the information could not be accepted. Review it and try again.",
        ),
        StatusCode::TOO_MANY_REQUESTS => (
            "Please wait a moment",
            "Too many requests were made. Please try again shortly.",
        ),
        StatusCode::SERVICE_UNAVAILABLE => (
            "Temporarily unavailable",
            "The service is temporarily unavailable. Please try again shortly.",
        ),
        _ => (
            "Something went wrong",
            "We couldn't complete your request. Please try again.",
        ),
    };

    (
        status,
        HttpErrorTemplate {
            app_name: APP_NAME,
            year: APP_YEAR,
            status_code: status.as_u16(),
            title,
            message,
        },
    )
        .into_response()
}
