use askama::Template;
use askama_web::WebTemplate;
use axum::{
    extract::State,
    response::{IntoResponse, Response},
};
use crate::{auth::CurrentUser, Context};
use crate::config::{APP_NAME, APP_YEAR};
use crate::routes::errors::not_found;

#[derive(Template, WebTemplate)]
#[template(path = "protected/profile.html")]
pub struct ProfileTemplate {
    pub app_name: &'static str,
    pub year: u16,
    pub organization_name: String,
    pub organization_profile: Profile,
}

pub struct Profile {
    pub display_name: String,
    // The name used in urls to represent this unique profile.
    pub slug: String,

    pub about: String,

    pub locations: Vec<Location>,
    pub links: Vec<ExternalLink>,

    pub logo: Option<Image>,
    pub banner: Option<Image>,
}

pub struct Location {
    pub city: String,
    pub country: String,
}

pub struct ExternalLink {
    pub label: String,
    pub url: String,
}

pub struct Image {
    pub url: String,
    pub alt: String,
}

//TODO check if there is need for unduplicating this load profile code (public/profile.rs and protected/profile.rs)
// there is a diffrence in verison as we use current here instead of publiched
pub async fn get_page(State(context): State<Context>, CurrentUser(user): CurrentUser) -> Response {
    let organization = match context.database.find_organization_by_id(user.organization.id).await {
        Ok(Some(organization)) => organization,
        Ok(None) => return not_found::get_page(),
        Err(error) => {
            tracing::error!(?error, "failed to load organization");
            return not_found::get_page();
        }
    };

    let profile = match context.database.find_profile_by_organization_id(organization.id).await {
        Ok(Some(profile)) => profile,
        Ok(None) => {
            tracing::error!(organization_id = organization.id,"organization exists without a profile");
            return not_found::get_page();
        }
        Err(error) => {
            tracing::error!(?error, "failed to load organization profile");
            return not_found::get_page();
        }
    };

    let current_version_id = match profile.current_version_id {
        Some(version_id) => version_id,
        None => {
            tracing::error!(profile_id = profile.id, "profile has no published version");
            return not_found::get_page();
        }
    };

    let profile_version = match context.database.find_profile_version_by_version_id(current_version_id).await {
        Ok(Some(profile_version)) => profile_version,
        Ok(None) => {
            tracing::error!(current_version_id = current_version_id,"profile references a version that does not exist");
            return not_found::get_page();
        }
        Err(error) => {
            tracing::error!(?error, "failed to load profile version");
            return not_found::get_page();
        }
    };

    // TODO: Replace these placeholder values with data loaded from the
    // organization profile.
    let profile = Profile {
        display_name: profile_version.display_name.clone(),

        slug: organization.slug.clone(),

        about: profile_version.about.clone(),

        locations: vec![
            Location {
                city: "Stockholm".to_string(),
                country: "Sweden".to_string(),
            },
        ],

        links: vec![
            ExternalLink {
                label: "Website".to_string(),
                url: "https://example.com".to_string(),
            },
            ExternalLink {
                label: "GitHub".to_string(),
                url: "https://github.com/example".to_string(),
            },
        ],

        logo: Some(Image {
            url: "/assets/test/logo.png".to_string(),
            alt: format!("{} logo", user.organization.name),
        }),

        banner: Some(Image {
            url: "/assets/test/banner.jpg".to_string(),
            alt: format!("{} banner", user.organization.name),
        }),
    };

    ProfileTemplate {
        app_name: APP_NAME,
        year: APP_YEAR,
        organization_name: user.organization.name,
        organization_profile: profile,
    }.into_response()
}