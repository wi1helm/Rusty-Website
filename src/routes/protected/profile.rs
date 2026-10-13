use askama::Template;
use askama_web::WebTemplate;
use axum::{
    extract::State,
    response::{IntoResponse, Response},
};
use crate::config::{APP_NAME, APP_YEAR};
use crate::routes::errors::not_found;
use crate::{auth::CurrentUser, Context};
use crate::routes::profile::{load_profile_view, ProfileView, VersionKind};

#[derive(Template, WebTemplate)]
#[template(path = "protected/profile.html")]
pub struct ProfileTemplate {
    pub app_name: &'static str,
    pub year: u16,
    pub organization_name: String,
    pub organization_profile: ProfileView,
}

pub async fn get_page(State(context): State<Context>, CurrentUser(user): CurrentUser) -> Response {
    let organization = match context.database.find_organization_by_organization_id(user.organization.id).await {
        Ok(Some(organization)) => organization,
        Ok(None) => return not_found::get_page(),
        Err(error) => {
            tracing::error!(?error, "failed to load organization");
            return not_found::get_page();
        }
    };

    let Some(profile) = load_profile_view(&context, &organization, VersionKind::Current).await else {
        return not_found::get_page();
    };

    ProfileTemplate {
        app_name: APP_NAME,
        year: APP_YEAR,
        organization_name: organization.name,
        organization_profile: profile,
    }
    .into_response()
}