use crate::config::{APP_NAME, APP_YEAR};
use crate::routes::errors::not_found;
use crate::routes::profile::ProfileView;
use crate::services::profile::ProfileService;
use askama::Template;
use askama_web::__askama_web_impl::axum_core_0_5::Response;
use askama_web::WebTemplate;
use axum::extract::{Path, State};
use axum::response::IntoResponse;
use std::sync::Arc;

#[derive(Template, WebTemplate)]
#[template(path = "public/profile.html")]
pub struct PublicProfileTemplate {
    pub app_name: &'static str,
    pub year: u16,
    pub profile_view: ProfileView,
}

pub async fn get_page(State(profile_service): State<Arc<ProfileService>>, Path(slug): Path<String>) -> Response {
    let Some(profile) = profile_service.load_by_slug(&slug).await
    else { return not_found::get_page(); };

    PublicProfileTemplate {
        app_name: APP_NAME,
        year: APP_YEAR,
        profile_view: profile,
    }
    .into_response()
}
