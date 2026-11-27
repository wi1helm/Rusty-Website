use crate::auth::CurrentUser;
use crate::config::{APP_NAME, APP_YEAR};
use crate::routes::errors::not_found;
use crate::routes::profile::ProfileView;
use crate::services::profile::ProfileService;
use askama::Template;
use askama_web::WebTemplate;
use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use std::sync::Arc;

#[derive(Template, WebTemplate)]
#[template(path = "protected/profile.html")]
pub struct ProfileTemplate {
    pub app_name: &'static str,
    pub year: u16,
    pub organization_name: String,
    pub profile_view: ProfileView,
    pub graph_json: String,
}

pub async fn get_page(State(profile_service): State<Arc<ProfileService>>, CurrentUser(user): CurrentUser) -> Response {
    let Some(profile) = profile_service.load_by_organization_id(user.organization.id).await else {
        return not_found::get_page();
    };

    let graph_json = match serde_json::to_string(&profile.graph) {
        Ok(json) => json.replace('<', "\\u003c").replace('>', "\\u003e").replace('&', "\\u0026"),

        Err(error) => {
            tracing::error!(?error, "failed to serialize organization graph");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    ProfileTemplate { app_name: APP_NAME, year: APP_YEAR, organization_name: profile.organization.name.clone(), profile_view: profile, graph_json }.into_response()
}
