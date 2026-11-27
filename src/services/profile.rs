use std::sync::Arc;

use serde::Serialize;
use serde_json::Value;
use uuid::Uuid;

use crate::storage::database::{DatabaseOrganization, OrganizationRepository, PositionRepository, ProfileRepository};
use crate::storage::object::ObjectStorage;

pub struct ProfileService {
    organizations: Arc<dyn OrganizationRepository>,
    profiles: Arc<dyn ProfileRepository>,
    positions: Arc<dyn PositionRepository>,
    object_storage: Arc<dyn ObjectStorage>,
}

impl ProfileService {
    pub fn new(organizations: Arc<dyn OrganizationRepository>, profiles: Arc<dyn ProfileRepository>, positions: Arc<dyn PositionRepository>, object_storage: Arc<dyn ObjectStorage>) -> Self {
        Self { organizations, profiles, positions, object_storage }
    }

    // TODO: Return repository failures separately from missing profiles; callers currently treat both as 404s.
    pub async fn load_by_slug(&self, slug: &str) -> Option<ProfileView> {
        let organization = match self.organizations.find_organization_by_slug(slug).await {
            Ok(Some(organization)) => organization,
            Ok(None) => return None,
            Err(error) => {
                tracing::error!(?error, slug, "failed to load organization");
                return None;
            }
        };

        self.load_profile(organization).await
    }

    pub async fn load_by_organization_id(&self, organization_id: i64) -> Option<ProfileView> {
        let organization = match self.organizations.find_organization_by_organization_id(organization_id).await {
            Ok(Some(organization)) => organization,
            Ok(None) => return None,
            Err(error) => {
                tracing::error!(?error, organization_id, "failed to load organization");
                return None;
            }
        };

        self.load_profile(organization).await
    }

    async fn load_profile(&self, organization: DatabaseOrganization) -> Option<ProfileView> {
        let profile = match self.profiles.find_profile_by_organization_id(organization.id).await {
            Ok(Some(profile)) => profile,
            Ok(None) => {
                tracing::error!(organization_id = organization.id, "organization exists without a profile");
                return None;
            }
            Err(error) => {
                tracing::error!(?error, organization_id = organization.id, "failed to load organization profile");
                return None;
            }
        };

        let positions = match self.positions.find_positions_by_organization_id(profile.organization_id).await {
            Ok(positions) => positions,
            Err(error) => {
                tracing::error!(?error, organization_id = profile.organization_id, "failed to load organization positions");
                return None;
            }
        };

        let connections = match self.positions.find_position_connections_by_organization_id(profile.organization_id).await {
            Ok(connections) => connections,
            Err(error) => {
                tracing::error!(?error, organization_id = profile.organization_id, "failed to load organization graph connections");
                return None;
            }
        };

        let links = profile.links.into_iter().map(|link| LinkView { label: link.label, url: link.url }).collect();
        let logo = ImageView { url: self.object_storage.url(&profile.logo.storage_key), alt: format!("{} logo", profile.display_name) };
        let banner = ImageView { url: self.object_storage.url(&profile.banner.storage_key), alt: format!("{} banner", profile.display_name) };
        let positions = positions.into_iter().map(|position| PositionView { id: position.id, title: position.title, description: position.description, current_status_id: position.current_status_id, statuses: position.statuses.into_iter().map(|status| StatusView { id: status.id, status: status.status.as_str().to_string(), information: status.information }).collect() }).collect();

        Some(ProfileView { organization: OrganizationView { name: organization.name, slug: organization.slug }, display_name: profile.display_name, about: profile.about, links, logo, banner, graph: GraphView { positions, connections: connections.into_iter().map(|connection| ConnectionView { from: connection.from, to: connection.to }).collect() } })
    }
}

pub struct OrganizationView {
    pub name: String,
    pub slug: String,
}

pub struct ProfileView {
    pub organization: OrganizationView,
    pub display_name: String,
    pub about: String,
    pub links: Vec<LinkView>,
    pub logo: ImageView,
    pub banner: ImageView,
    pub graph: GraphView,
}

pub struct LinkView {
    pub label: String,
    pub url: String,
}

pub struct ImageView {
    pub url: String,
    pub alt: String,
}

#[derive(Debug, Serialize)]
pub struct GraphView {
    pub positions: Vec<PositionView>,
    pub connections: Vec<ConnectionView>,
}

#[derive(Debug, Serialize)]
pub struct ConnectionView {
    pub from: Uuid,
    pub to: Uuid,
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
