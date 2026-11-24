use crate::auth::CurrentUser;
use crate::config::{APP_NAME, APP_YEAR};
use crate::routes::errors::not_found;
use crate::routes::profile::{ProfileView, load_profile_view};
use crate::storage::database::Database;
use crate::storage::object::ObjectStorage;
use askama::Template;
use askama_web::WebTemplate;
use axum::{
    extract::State,
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

pub async fn get_page(State(database): State<Arc<dyn Database>>, State(object_storage): State<Arc<dyn ObjectStorage>>, CurrentUser(user): CurrentUser) -> Response {
    let organization = match database.find_organization_by_organization_id(user.organization.id).await {
        Ok(Some(organization)) => organization,
        Ok(None) => return not_found::get_page(),
        Err(error) => {
            tracing::error!(?error, "failed to load organization");
            return not_found::get_page();
        }
    };

    let Some(profile) = load_profile_view(database.as_ref(), object_storage.as_ref(), &organization).await
    else { return not_found::get_page(); };

    let graph_json = match serde_json::to_string(&profile.graph) {
        Ok(json) => json
            .replace('<', "\\u003c")
            .replace('>', "\\u003e")
            .replace('&', "\\u0026"),

        Err(error) => {
            tracing::error!(?error, "failed to serialize organization graph");
            return not_found::get_page();
        }
    };

    ProfileTemplate {
        app_name: APP_NAME,
        year: APP_YEAR,
        organization_name: organization.name.clone(),
        profile_view: profile,
        graph_json,
    }
    .into_response()
}
