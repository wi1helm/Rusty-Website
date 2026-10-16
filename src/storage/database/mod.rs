// database/mod.rs
pub mod postgres;

use async_trait::async_trait;
use sqlx::types::chrono::{DateTime, Utc};

#[derive(Debug, thiserror::Error)]
pub enum DatabaseError {
    #[error("database error")]
    Sqlx(#[from] sqlx::Error),

    #[error("invalid profile image type in database: {0}")]
    InvalidProfileImageType(String),
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProfileImageType {
    Logo,
    Banner,
}
pub struct ImageRules {
    // Validation
    pub min_width: u32,
    pub min_height: u32,
    pub max_width: u32,
    pub max_height: u32,
    pub max_pixels: u64,
    pub min_aspect_ratio: f64,
    pub max_aspect_ratio: f64,

    // Normalization
    pub output_max_width: u32,
    pub output_max_height: u32,
}
impl ProfileImageType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ProfileImageType::Logo => "logo",
            ProfileImageType::Banner => "banner",
        }
    }
    pub fn from_str(value: &str) -> Option<Self> {
        match value {
            "logo" => Some(Self::Logo),
            "banner" => Some(Self::Banner),
            _ => None,
        }
    }
    pub fn rules(&self) -> ImageRules {
        match self {
            Self::Logo => ImageRules {
                min_width: 200,
                min_height: 200,
                max_width: 5000,
                max_height: 5000,
                max_pixels: 25_000_000,
                min_aspect_ratio: 0.75,
                max_aspect_ratio: 1.33,
                output_max_width: 1024,
                output_max_height: 1024,
            },

            Self::Banner => ImageRules {
                min_width: 800,
                min_height: 200,
                max_width: 5000,
                max_height: 2500,
                max_pixels: 25_000_000,
                min_aspect_ratio: 2.0,
                max_aspect_ratio: 5.0,
                output_max_width: 2048,
                output_max_height: 1024,
            },
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

    pub links: Vec<NewProfileLink>,
}
//TODO think if profiles images should be tied to profile versions as they are not cared by or interacting with versions.
pub struct NewProfileImage {
    pub profile_id: i64,
    pub image_type: ProfileImageType,
    pub storage_key: String,
    pub content_type: String,
    pub width: i32,
    pub height: i32,
    pub file_size: i64,
}
pub struct NewProfileLink {
    pub label: String,
    pub url: String,
    pub position: i32,
}

pub struct DatabaseOrganization {
    pub id: i64,
    pub name: String,
    pub slug: String,
    pub created_at: DateTime<Utc>,
}
pub struct DatabaseMember {
    pub id: i64,
    pub organization_id: i64,
    pub email: String,
    pub password_hash: String,
    pub created_at: DateTime<Utc>,
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
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
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
    pub created_at: DateTime<Utc>,
    pub published_at: Option<DateTime<Utc>>,
}

pub struct DatabaseImage {
    pub id: i64,
    pub profile_id: i64,
    pub image_type: ProfileImageType,
    pub storage_key: String,
    pub content_type: String,
    pub width: i32,
    pub height: i32,
    pub file_size: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
pub struct DatabaseProfileLink {
    pub id: i64,
    pub profile_version_id: i64,
    pub label: String,
    pub url: String,
    pub position: i32,
    pub created_at: DateTime<Utc>,
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
    async fn find_profile_image(&self, profile_id: i64, image_type: ProfileImageType) -> Result<Option<DatabaseImage>, DatabaseError>;
    async fn save_profile_image(&self, image: NewProfileImage) -> Result<Option<DatabaseImage>, DatabaseError>;
    async fn delete_profile_image(&self, profile_id: i64, image_type: ProfileImageType) -> Result<Option<DatabaseImage>, DatabaseError>;

    async fn find_profile_links_by_version_id(&self, version_id: i64) -> Result<Vec<DatabaseProfileLink>, DatabaseError>;
}