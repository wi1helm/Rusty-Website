// database/mod.rs

mod errors;
mod data;
pub mod postgres;
mod mapping;

pub use errors::{
    CreateOrganizationError,
    DatabaseError,
};

pub use data::{
    DatabaseImage,
    DatabaseMember,
    DatabaseMemberWithOrganization,
    DatabaseOrganization,
    DatabaseProfile,
    DatabaseLink,
    NewMember,
    NewOrganization,
    NewProfileImage,
    NewProfileLink,
    NewProfile,
    UpdateProfile,
    OrganizationRole,
    ProfileImageType,
};

use async_trait::async_trait;
use crate::storage::database::data::DatabasePosition;

#[async_trait]
pub trait Database: Send + Sync {
    /// Creates a new organization, assigns the provided member the owner role,
    /// and creates a profile with default information.
    ///
    /// Returns the IDs of the created organization and owner member, respectively.
    /// Returns an error if the organization, owner, or profile cannot be created.
    async fn create_organization_with_owner_and_profile(&self, organization: NewOrganization, owner: NewMember) -> Result<(i64, i64), CreateOrganizationError>;
    ///
    /// Finds an organization by its unique ID.
    ///
    /// Returns `Some(organization)` if the organization exists, or `None` if it does not.
    /// Returns an error if the database operation fails.
    async fn find_organization_by_organization_id(&self, organization_id: i64) -> Result<Option<DatabaseOrganization>, DatabaseError>;
    ///
    /// Finds an organization by its unique slug.
    ///
    /// Returns `Some(organization)` if the organization exists, or `None` if it does not.
    /// Returns an error if the database operation fails.
    async fn find_organization_by_slug(&self, slug: &str) -> Result<Option<DatabaseOrganization>, DatabaseError>;
    ///
    /// Finds a member by their email address.
    ///
    /// Returns `Some(member)` if a matching member exists, or `None` if no member is found.
    /// Returns an error if the database operation fails.
    async fn find_member_by_email(&self, email: &str)-> Result<Option<DatabaseMember>, DatabaseError>;
    ///
    /// Finds a member by their ID and includes information about their organization.
    ///
    /// Returns `Some(member)` with the associated organization if the member exists,
    /// or `None` if no matching member is found.
    /// Returns an error if the database operation fails.
    async fn find_member_with_organization_by_member_id(&self, member_id: i64) -> Result<Option<DatabaseMemberWithOrganization>, DatabaseError>;
    ///
    /// Finds the profile associated with the specified organization.
    ///
    /// Returns `Some(profile)` if the organization has a profile, or `None` if no profile is found.
    /// Returns an error if the database operation fails.
    async fn find_profile_by_organization_id(&self, organization_id: i64) -> Result<Option<DatabaseProfile>, DatabaseError>;
    ///
    /// Finds a list of positions associated with the specified organization.
    ///
    /// Returns `Vec<DatabasePosition>` empty means no positions for this organization.
    /// Returns an error if the database operation fails.
    async fn find_positions_by_organization_id(&self, organization_id: i64) -> Result<Vec<DatabasePosition>, DatabaseError>;

    ///
    /// Updates the profile associated with the specified organization.
    ///
    /// The member ID identifies the member performing the update.
    /// Returns `Some(profile)` if the profile is updated, or `None` if no matching profile is found.
    /// Returns an error if the database operation fails or the update cannot be completed.
    async fn update_profile(&self, organization_id: i64, member_id: i64, profile: UpdateProfile) -> Result<Option<DatabaseProfile>, DatabaseError>;
    ///
    /// Updates the profile image using the provided image information.
    ///
    /// Returns `Some(image)` if the image is updated, or `None` if the target profile or image cannot be found.
    /// Returns an error if the database operation fails.
    async fn update_profile_image(&self, image: NewProfileImage) -> Result<Option<DatabaseImage>, DatabaseError>;
    ///
    /// Updates the profile links for the specified organization.
    ///
    /// Replaces or updates the links according to the database implementation.
    /// Returns the resulting collection of profile links.
    /// Returns an error if the database operation fails.
    async fn update_profile_links(&self, organization_id: i64, links: Vec<NewProfileLink>) -> Result<Vec<DatabaseLink>, DatabaseError>;
}