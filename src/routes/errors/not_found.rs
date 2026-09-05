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
    NotFoundTemplate {
        app_name: APP_NAME,
        year: APP_YEAR,
    }.into_response()
}