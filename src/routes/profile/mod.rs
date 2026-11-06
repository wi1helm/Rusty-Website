//! Loads everything needed to render an organization profile page.
//!
//! Converts database models into view models so page handlers and
//! templates don't need to depend on database-specific structs.

use crate::storage::database::{
    DatabaseOrganization,
    DatabaseProfile,
};
use crate::Context;

/// The initial graph version until graph versioning is implemented.
const INITIAL_GRAPH_VERSION: i64 = 1;

/// Information about the organization displayed on its profile page.
pub struct OrganizationView {
    pub name: String,
    pub slug: String,
}

/// Information needed to render an organization's profile page.
pub struct ProfileView {
    pub organization: OrganizationView,
    pub display_name: String,
    pub about: String,
    pub links: Vec<LinkView>,
    pub logo: ImageView,
    pub banner: ImageView,

    /// The version of the organization's graph.
    pub graph_version: i64,
}

/// A link displayed on the organization profile.
pub struct LinkView {
    pub label: String,
    pub url: String,
}

/// An image displayed on the organization profile.
pub struct ImageView {
    pub url: String,
    pub alt: String,
}

/// Loads the information needed to render an organization profile.
///
/// Returns `None` if the profile cannot be loaded.
/// The reason is logged, and callers should respond with a not-found page.
pub async fn load_profile_view(context: &Context, organization: &DatabaseOrganization) -> Option<ProfileView> {
    let profile = match context.database.find_profile_by_organization_id(organization.id).await {
        Ok(Some(profile)) => profile,

        Ok(None) => {
            tracing::error!(organization_id = organization.id,"organization exists without a profile");
            return None;
        }

        Err(error) => {
            tracing::error!(?error, organization_id = organization.id, "failed to load organization profile");
            return None;
        }
    };


    let organization = OrganizationView {
        name: organization.name.clone(),
        slug: organization.slug.clone(),
    };

    let links = profile
        .links
        .into_iter()
        .map(|link| LinkView {
            label: link.label,
            url: link.url,
        })
        .collect();

    let logo = ImageView {
        url: context.public_object_storage.url(&profile.logo.storage_key),
        alt: format!("{} logo", profile.display_name),
    };

    let banner = ImageView {
        url: context.public_object_storage.url(&profile.banner.storage_key),
        alt: format!("{} banner", profile.display_name),
    };

    let profile_view = ProfileView {
        organization,
        display_name: profile.display_name,
        about: profile.about,
        links,
        logo,
        banner,
        graph_version: INITIAL_GRAPH_VERSION,
    };

    Some(profile_view)
}
