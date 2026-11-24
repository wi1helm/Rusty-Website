use askama::Template;
use askama_web::WebTemplate;
use axum::response::{IntoResponse, Response};

use crate::config::{APP_NAME, APP_YEAR};

#[derive(Template, WebTemplate)]
#[template(path = "errors/not_found.html")]
pub struct NotFoundTemplate {
    pub app_name: &'static str,
    pub year: u16,
}

pub fn get_page() -> Response {
    // TODO: Return HTTP 404 status code instead of 200 OK. Currently .into_response()
    // produces a 200 status, which is incorrect HTTP semantics and can cause SEO/caching
    // issues. Use (StatusCode::NOT_FOUND, template).into_response() or similar.
    // See: https://developer.mozilla.org/en-US/docs/Web/HTTP/Status/404
    NotFoundTemplate {
        app_name: APP_NAME,
        year: APP_YEAR,
    }
    .into_response()
}
