use crate::config::{APP_NAME, APP_YEAR};
use crate::routes::errors::not_found;
use crate::routes::profile::{ProfileView, load_profile_view};
use crate::storage::database::Database;
use crate::storage::object::ObjectStorage;
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

pub async fn get_page(State(database): State<Arc<dyn Database>>, State(object_storage): State<Arc<dyn ObjectStorage>>, Path(slug): Path<String>, ) -> Response {
    let organization = match database.find_organization_by_slug(&slug).await {
        Ok(Some(organization)) => organization,
        Ok(None) => return not_found::get_page(),
        Err(error) => {
            tracing::error!(?error, "failed to load organization");
            return not_found::get_page();
        }
    };

    let Some(profile) = load_profile_view(database.as_ref(), object_storage.as_ref(), &organization).await
    else { return not_found::get_page(); };

    PublicProfileTemplate {
        app_name: APP_NAME,
        year: APP_YEAR,
        profile_view: profile,
    }
    .into_response()
}
