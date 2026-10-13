use askama::Template;
use askama_web::__askama_web_impl::axum_core_0_5::Response;
use askama_web::WebTemplate;
use axum::extract::{Path, State};
use axum::response::IntoResponse;
use crate::config::{APP_NAME, APP_YEAR};
use crate::routes::errors::not_found;
use crate::Context;
use crate::routes::profile::{load_profile_view, ProfileView, VersionKind};

#[derive(Template, WebTemplate)]
#[template(path = "public/profile.html")]
pub struct PublicProfileTemplate {
    pub app_name: &'static str,
    pub year: u16,
    pub organization_name: String,
    pub organization_profile: ProfileView,
}

pub async fn get_page(State(context): State<Context>, Path(slug): Path<String>) -> Response {
    let organization = match context.database.find_organization_by_slug(&slug).await {
        Ok(Some(organization)) => organization,
        Ok(None) => return not_found::get_page(),
        Err(error) => {
            tracing::error!(?error, "failed to load organization");
            return not_found::get_page();
        }
    };

    let Some(profile) = load_profile_view(&context, &organization, VersionKind::Published).await else {
        return not_found::get_page();
    };

    PublicProfileTemplate {
        app_name: APP_NAME,
        year: APP_YEAR,
        organization_name: organization.name,
        organization_profile: profile,
    }
    .into_response()
}