//! Loads everything needed to render an organization profile page.
//!
//! Shared by the public page (published version) and the protected
//! editor page (current/draft version). The only difference between
//! them is which version is loaded, see `VersionKind`.
//!
//! Register in main.rs / lib.rs with:  `mod profile_view;`

use crate::config::{DEFAULT_BANNER_KEY, DEFAULT_LOGO_KEY};
use crate::storage::database::{DatabaseOrganization, ProfileImageType}; // adjust Organization's path if needed
use crate::Context;

/// Which version of the profile to show.
#[derive(Clone, Copy)]
pub enum VersionKind {
    /// What the public sees.
    Published,
    /// The editable draft.
    Current,
}

pub struct ProfileView {
    pub display_name: String,
    /// The name used in urls to represent this unique profile.
    pub slug: String,
    pub version: i32,
    pub about: String,
    pub links: Vec<ExternalLink>,
    pub logo: Image,
    pub banner: Image,
}

pub struct ExternalLink {
    pub label: String,
    pub url: String,
}

pub struct Image {
    pub url: String,
    pub alt: String,
}

/// Returns `None` if anything is missing or fails. The reason is logged,
/// and callers should respond with a not-found page.
pub async fn load_profile_view(context: &Context, organization: &DatabaseOrganization, kind: VersionKind) -> Option<ProfileView> {
    let profile = match context.database.find_profile_by_organization_id(organization.id).await {
        Ok(Some(profile)) => profile,
        Ok(None) => {
            tracing::error!(organization_id = organization.id, "organization exists without a profile");
            return None;
        }
        Err(error) => {
            tracing::error!(?error, "failed to load organization profile");
            return None;
        }
    };

    let version_id = match kind {
        VersionKind::Published => profile.published_version_id,
        VersionKind::Current => profile.current_version_id,
    };

    let Some(version_id) = version_id else {
        tracing::error!(profile_id = profile.id, "profile has no version for the requested kind");
        return None;
    };

    let profile_version = match context.database.find_profile_version_by_version_id(version_id).await {
        Ok(Some(version)) => version,
        Ok(None) => {
            tracing::error!(profile_version_id = version_id, "profile references a version that does not exist");
            return None;
        }
        Err(error) => {
            tracing::error!(?error, "failed to load profile version");
            return None;
        }
    };

    let links = match context.database.find_profile_links_by_version_id(version_id).await {
        Ok(links) => links
            .into_iter()
            .map(|link| ExternalLink { label: link.label, url: link.url })
            .collect(),
        Err(error) => {
            tracing::error!(?error, profile_version_id = version_id, "failed to load profile links");
            return None;
        }
    };

    let logo = load_image(context, profile.id, ProfileImageType::Logo, DEFAULT_LOGO_KEY, "logo", &organization.name).await;
    let banner = load_image(context, profile.id, ProfileImageType::Banner, DEFAULT_BANNER_KEY, "banner", &organization.name).await;

    Some(ProfileView {
        display_name: profile_version.display_name,
        slug: organization.slug.clone(),
        version: profile_version.version,
        about: profile_version.about,
        links,
        logo,
        banner,
    })
}

/// Loads an image, falling back to the default on "none" or on error.
async fn load_image(context: &Context, profile_id: i64, image_type: ProfileImageType, default_key: &str, label: &str, organization_name: &str) -> Image {
    let alt = format!("{organization_name} {label}");

    let key = match context.database.find_profile_image(profile_id, image_type).await {
        Ok(Some(image)) => image.storage_key,
        Ok(None) => default_key.to_string(),
        Err(error) => {
            tracing::error!(?error, profile_id, image_type = label, "failed to load profile image");
            default_key.to_string()
        }
    };

    Image {
        url: context.public_object_storage.url(&key),
        alt,
    }
}