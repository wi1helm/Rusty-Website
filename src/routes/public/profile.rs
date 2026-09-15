use askama::Template;
use askama_web::__askama_web_impl::axum_core_0_5::Response;
use askama_web::WebTemplate;
use axum::extract::{Path, State};
use axum::response::IntoResponse;
use crate::config::{APP_NAME, APP_YEAR, DEFAULT_BANNER_KEY, DEFAULT_LOGO_KEY};
use crate::Context;
use crate::routes::errors::not_found;
use crate::storage::database::ProfileImageType;

#[derive(Template, WebTemplate)]
#[template(path = "public/profile.html")]
pub struct PublicProfileTemplate {
    pub app_name: &'static str,
    pub year: u16,
    pub organization_profile: PublicProfile,
}

pub struct PublicProfile {
    pub display_name: String,
    // The name used in urls to represent this unique profile.
    pub slug: String,

    pub about: String,

    pub locations: Vec<Location>,
    pub links: Vec<ExternalLink>,

    pub logo: Image,
    pub banner: Image,
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


pub async fn get_page(State(context): State<Context>, Path(slug): Path<String>) -> Response {
    let organization = match context.database.find_organization_by_slug(&slug).await {
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

    let published_version_id = match profile.published_version_id {
        Some(version_id) => version_id,
        None => {
            tracing::error!(profile_id = profile.id, "profile has no published version");
            return not_found::get_page();
        }
    };

    let profile_version = match context.database.find_profile_version_by_version_id(published_version_id).await {
        Ok(Some(profile_version)) => profile_version,
        Ok(None) => {
            tracing::error!(profile_version_id = published_version_id,"profile references a version that does not exist");
            return not_found::get_page();
        }
        Err(error) => {
            tracing::error!(?error, "failed to load profile version");
            return not_found::get_page();
        }
    };

    let logo = match context.database.find_profile_image(profile.id, ProfileImageType::Logo).await {
        Ok(Some(image)) => Image {
            url: context.public_object_storage.url(&image.storage_key),
            alt: format!("{} logo", organization.name),
        },
        Ok(None) => Image {
            url: context.public_object_storage.url(DEFAULT_LOGO_KEY),
            alt: format!("{} logo", organization.name),
        },
        Err(error) => {
            tracing::error!(?error, profile_id = profile.id, image_type = "logo", "failed to load profile logo");
            Image {
                url: context.public_object_storage.url(DEFAULT_LOGO_KEY),
                alt: format!("{} logo", organization.name),
            }
        }
    };

    let banner = match context.database.find_profile_image(profile.id, ProfileImageType::Banner).await {
        Ok(Some(image)) => Image {
            url: context.public_object_storage.url(&image.storage_key),
            alt: format!("{} banner", organization.name),
        },
        Ok(None) => Image {
            url: context.public_object_storage.url(DEFAULT_BANNER_KEY),
            alt: format!("{} banner", organization.name),
        },
        Err(error) => {
            tracing::error!(?error, profile_id = profile.id, image_type = "banner", "failed to load profile banner");
            Image {
                url: context.public_object_storage.url(DEFAULT_BANNER_KEY),
                alt: format!("{} banner", organization.name),
            }
        }
    };

    // TODO: Replace these placeholder values with data loaded from the
    // organization profile.
    let public_profile = PublicProfile {
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

        logo,
        banner
    };

    PublicProfileTemplate {
        app_name: APP_NAME,
        year: APP_YEAR,
        organization_profile: public_profile,
    }.into_response()
}