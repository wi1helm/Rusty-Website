use askama::Template;
use askama_web::WebTemplate;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};

use crate::config::{APP_NAME, APP_YEAR};

#[derive(Template, WebTemplate)]
#[template(path = "errors/not_found.html")]
pub struct NotFoundTemplate {
    pub app_name: &'static str,
    pub year: u16,
}

pub fn get_page() -> Response {
    (StatusCode::NOT_FOUND, NotFoundTemplate { app_name: APP_NAME, year: APP_YEAR }).into_response()
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::StatusCode;

    #[test]
    fn not_found_page_returns_not_found_status() {
        assert_eq!(get_page().status(), StatusCode::NOT_FOUND);
    }
}
