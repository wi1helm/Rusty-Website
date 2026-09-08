// database/mod.rs
pub mod postgres;

use async_trait::async_trait;

#[derive(Debug, thiserror::Error)]
pub enum DatabaseError {
    #[error("database error")]
    Sqlx(#[from] sqlx::Error),
}

#[derive(Debug, thiserror::Error)]
pub enum CreateOrganizationError {
    #[error("organization slug already exists")]
    OrganizationSlugAlreadyExists,

    #[error("member already exists")]
    MemberAlreadyExists,

    #[error("database error")]
    Database(#[from] DatabaseError),
}
impl From<sqlx::Error> for CreateOrganizationError {
    fn from(error: sqlx::Error) -> Self {
        CreateOrganizationError::Database(DatabaseError::Sqlx(error))
    }
}
#[derive(Debug, thiserror::Error)]
pub enum PublishProfileError {
    #[error("profile not found")]
    ProfileNotFound,

    #[error("profile has no current version")]
    ProfileHasNoCurrentVersion,

    #[error("database error")]
    Database(#[from] DatabaseError),
}
impl From<sqlx::Error> for PublishProfileError {
    fn from(error: sqlx::Error) -> Self {
        PublishProfileError::Database(DatabaseError::Sqlx(error))
    }
}

pub enum OrganizationRoles{
    Owner,
    Recruiter,
    Member,
}
impl OrganizationRoles {
    pub fn as_str(&self) -> &'static str {
        match self {
            OrganizationRoles::Owner => "owner",
            OrganizationRoles::Recruiter => "recruiter",
            OrganizationRoles::Member => "member",
        }
    }
}

pub struct NewOrganization {
    pub name: String,
    pub slug: String,
}

pub struct NewMember {
    pub email: String,
    pub password_hash: String, //TODO add hashing
    pub role: OrganizationRoles,
}

pub struct NewProfileVersion {
    pub profile_id: i64,
    pub version: i32,
    pub display_name: String,
    pub about: String,
    pub created_by_member_id: i64,
}

pub struct DatabaseOrganization {
    pub id: i64,
    pub name: String,
    pub slug: String,
}
pub struct DatabaseMember {
    pub id: i64,
    pub organization_id: i64,
    pub email: String,
    pub password_hash: String,
}
pub struct DatabaseMemberWithOrganization {
    pub member: DatabaseMember,
    pub organization: DatabaseOrganization,
}
pub struct DatabaseProfile {
    pub id: i64,
    pub organization_id: i64,
    pub current_version_id: Option<i64>,
    pub published_version_id: Option<i64>,
}
/// DatabaseProfileVersion contains the content of the a verison of the profile what version is valid
/// given by the DatabaseProfile struct.
pub struct DatabaseProfileVersion {
    pub id: i64,
    pub profile_id: i64,
    pub version: i32,
    pub display_name: String,
    pub about: String,
    pub created_by_member_id: i64,
}

pub struct DatabaseImage {
    pub id: i64,
    pub organization_id: i64,

    pub storage_key: String,
    pub content_type: String,

    pub width: i32,
    pub height: i32,
    pub file_size: i64,
}

#[async_trait]
pub trait Database: Send + Sync {
    /// The long name "create_organization_with_owner_and_profile" exists to communicate to implementations that both owner and profile
    /// NEEEDS! to be created. + this is only used on signup really menaing we return id for organization and owener not struct.
    async fn create_organization_with_owner_and_profile(&self, organization: NewOrganization, owner: NewMember) -> Result<(i64, i64), CreateOrganizationError>;
    // Returns the new profile_version_id
    async fn create_new_profile_version(&self, profile_version: NewProfileVersion) -> Result<i64, DatabaseError>;
    async fn find_member_by_email(&self, email: &str) -> Result<Option<DatabaseMember>, DatabaseError>;
    // Option<Organization> rather than just Organization,
    // because an ID from a session might theoretically
    // refer to an organization that no longer exists.
    async fn find_organization_by_organization_id(&self, organization_id: i64) -> Result<Option<DatabaseOrganization>, DatabaseError>;
    async fn find_member_with_organization_by_member_id(&self, member_id: i64) -> Result<Option<DatabaseMemberWithOrganization>, DatabaseError>;
    async fn find_organization_by_slug(&self, slug: &str) -> Result<Option<DatabaseOrganization>, DatabaseError>;

    async fn find_profile_by_organization_id(&self, organization_id: i64) -> Result<Option<DatabaseProfile>, DatabaseError>;
    async fn find_profile_version_by_version_id(&self, version_id: i64) -> Result<Option<DatabaseProfileVersion>, DatabaseError>;

    async fn publish_current_profile_version(&self, profile_id: i64) -> Result<(), PublishProfileError>;
}