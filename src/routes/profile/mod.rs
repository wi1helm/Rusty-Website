//! Loads everything needed to render an organization profile page.
//!
//! Converts database models into view models so page handlers and
//! templates don't need to depend on database-specific structs.

use uuid::Uuid;
use crate::storage::database::{
    DatabaseOrganization,
    DatabaseProfile,
};
use crate::Context;
use serde::Serialize;
use serde_json::Value;

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

    pub graph: GraphView,
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

#[derive(Debug, Serialize)]
pub struct GraphView {
    pub positions: Vec<PositionView>,
}

#[derive(Debug, Serialize)]
pub struct PositionView {
    pub id: Uuid,
    pub title: String,
    pub description: String,

    pub current_status_id: Uuid,
    pub statuses: Vec<StatusView>,
}

#[derive(Debug, Serialize)]
pub struct StatusView {
    pub id: Uuid,
    pub status: String,
    pub information: Value,
}

pub struct ConnectionView {
    pub from: Uuid,
    pub to: Uuid,
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

    let positions = match context.database.find_positions_by_organization_id(profile.organization_id).await {
        Ok(positions) => positions,

        Err(error) => {
            tracing::error!(?error, organization_id = profile.organization_id, "failed to load organization positions");
            return None;
        }
    };

    let positions: Vec<PositionView> = positions
        .into_iter()
        .map(|position| PositionView {
            id: position.id,
            title: position.title,
            description: position.description,
            current_status_id: position.current_status_id,

            statuses: position
                .statuses
                .into_iter()
                .map(|status| StatusView {
                    id: status.id,
                    status: status.status.as_str().to_string(),
                    information: status.information,
                })
                .collect(),
        })
        .collect();

    let profile_view = ProfileView {
        organization,
        display_name: profile.display_name,
        about: profile.about,
        links,
        logo,
        banner,
        graph: GraphView {
            positions,
        },
    };

    Some(profile_view)
}
