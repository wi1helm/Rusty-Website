// database/data.rs

use sqlx::types::chrono::{DateTime, Utc};

/// Represents the information required to create a new organization.
#[derive(Debug, Clone)]
pub struct NewOrganization {
    /// The display name of the organization.
    pub name: String,
    /// The unique URL-friendly identifier used to identify the organization.
    pub slug: String,
}

/// Defines the roles a member can have within an organization.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrganizationRole {
    /// Has full control over the organization.
    Owner,
    /// Can manage the organization with administrative permissions.
    Admin,
    /// Has standard member permissions within the organization.
    Member,
}

impl OrganizationRole {
    /// Returns the lowercase string representation of the role.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Owner => "owner",
            Self::Admin => "admin",
            Self::Member => "member",
        }
    }

    /// Converts a string into an organization role.
    ///
    /// Returns `Some(role)` if the string matches a supported role,
    /// or `None` if the value is invalid.
    pub fn from_str(value: &str) -> Option<Self> {
        match value {
            "owner" => Some(Self::Owner),
            "admin" => Some(Self::Admin),
            "member" => Some(Self::Member),
            _ => None,
        }
    }
}

/// Represents the information required to create a new organization member.
#[derive(Debug, Clone)]
pub struct NewMember {
    /// The email address used to identify the member.
    pub email: String,
    /// The hashed password used to authenticate the member.
    pub password_hash: String,
    /// The role assigned to the member within the organization.
    pub role: OrganizationRole,
}

/// Represents an organization retrieved from the database.
#[derive(Debug, Clone)]
pub struct DatabaseOrganization {
    /// The unique identifier of the organization.
    pub id: i64,
    /// The display name of the organization.
    pub name: String,
    /// The unique URL-friendly identifier of the organization.
    pub slug: String,
    /// The timestamp when the organization was created.
    pub created_at: DateTime<Utc>,
}

/// Represents a member retrieved from the database.
#[derive(Debug, Clone)]
pub struct DatabaseMember {
    /// The unique identifier of the member.
    pub id: i64,
    /// The identifier of the organization the member belongs to.
    pub organization_id: i64,
    /// The member's email address.
    pub email: String,
    /// The hashed password used to authenticate the member.
    pub password_hash: String,
    /// The member's role within the organization.
    pub role: OrganizationRole,
    /// The timestamp when the member was created.
    pub created_at: DateTime<Utc>,
    /// The timestamp when the member was last updated.
    pub updated_at: DateTime<Utc>,
}

/// Represents a member together with the organization they belong to.
#[derive(Debug, Clone)]
pub struct DatabaseMemberWithOrganization {
    /// The member's database record.
    pub member: DatabaseMember,
    /// The database record of the member's organization.
    pub organization: DatabaseOrganization,
}

/// Represents the information used to create an organization profile.
#[derive(Debug, Clone)]
pub struct NewProfile {
    /// The display name of the profile, if provided.
    pub display_name: String,
}
/// Represents the information used to update an organization profile.
#[derive(Debug, Clone)]
pub struct UpdateProfile {
    //TODO try to make these optional so we dont make database writes
    // when not needed. So a field empty dont process it. However its harder then you think to make it read good
    /// The display name of the profile, if provided.
    pub display_name: String,
    /// The descriptive text for the profile, if provided.
    pub about: String
}
/// Represents an organization profile retrieved from the database.
#[derive(Debug, Clone)]
pub struct DatabaseProfile {
    /// The unique identifier of the profile.
    pub id: i64,
    /// The identifier of the organization associated with the profile.
    pub organization_id: i64,
    /// The display name shown on the profile.
    pub display_name: String,
    /// The descriptive text displayed on the profile.
    pub about: String,
    /// The logo image associated with the profile.
    pub logo: DatabaseImage,
    /// The banner image associated with the profile.
    pub banner: DatabaseImage,
    /// The links associated with the profile.
    pub links: Vec<DatabaseLink>,

    /// The identifier of the member who last updated the profile.
    pub updated_by_member_id: i64,
    /// The timestamp when the profile was created.
    pub created_at: DateTime<Utc>,
    /// The timestamp when the profile was last updated.
    pub updated_at: DateTime<Utc>,
}

/// Defines the supported image types used by an organization profile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProfileImageType {
    /// Represents the organization's logo image.
    Logo,
    /// Represents the organization's banner image.
    Banner,
}

impl ProfileImageType {
    /// Returns the lowercase string representation of the image type.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Logo => "logo",
            Self::Banner => "banner",
        }
    }

    /// Converts a string into a profile image type.
    ///
    /// Returns `Some(image_type)` if the string matches a supported image type,
    /// or `None` if the value is invalid.
    pub fn from_str(value: &str) -> Option<Self> {
        match value {
            "logo" => Some(Self::Logo),
            "banner" => Some(Self::Banner),
            _ => None,
        }
    }
}

/// Defines the validation and output constraints for a profile image type.
pub struct ImageRules {
    /// The minimum permitted image width in pixels.
    pub min_width: u32,
    /// The minimum permitted image height in pixels.
    pub min_height: u32,
    /// The maximum permitted image width in pixels.
    pub max_width: u32,
    /// The maximum permitted image height in pixels.
    pub max_height: u32,
    /// The maximum permitted total number of pixels in the image.
    pub max_pixels: u64,
    /// The minimum permitted aspect ratio, calculated as width divided by height.
    pub min_aspect_ratio: f64,
    /// The maximum permitted aspect ratio, calculated as width divided by height.
    pub max_aspect_ratio: f64,
    /// The maximum width of the processed output image in pixels.
    pub output_max_width: u32,
    /// The maximum height of the processed output image in pixels.
    pub output_max_height: u32,
}

impl ProfileImageType {
    /// Returns the validation and output rules for this image type.
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

/// Represents the information required to create or update a profile image.
#[derive(Debug, Clone)]
pub struct NewProfileImage {
    /// The identifier of the organization the image belongs to.
    pub organization_id: i64,
    /// The type of profile image being stored.
    pub image_type: ProfileImageType,
    /// The storage key used to locate the image in the file storage system.
    pub storage_key: String,
    /// The MIME content type of the image, such as `image/png`.
    pub content_type: String,
    /// The width of the image in pixels.
    pub width: i32,
    /// The height of the image in pixels.
    pub height: i32,
    /// The size of the image file in bytes.
    pub file_size: i64,
}

/// Represents a profile image retrieved from the database.
#[derive(Debug, Clone)]
pub struct DatabaseImage {
    /// The unique identifier of the image.
    pub id: i64,
    /// The identifier of the organization the image belongs to.
    pub organization_id: i64,
    /// The type of profile image.
    pub image_type: ProfileImageType,
    /// The storage key used to locate the image in the file storage system.
    pub storage_key: String,
    /// The MIME content type of the image, such as `image/png`.
    pub content_type: String,
    /// The width of the image in pixels.
    pub width: i32,
    /// The height of the image in pixels.
    pub height: i32,
    /// The size of the image file in bytes.
    pub file_size: i64,
    /// The timestamp when the image record was created.
    pub created_at: DateTime<Utc>,
    /// The timestamp when the image record was last updated.
    pub updated_at: DateTime<Utc>,
}

/// Represents the information required to create or update a profile link.
#[derive(Debug, Clone)]
pub struct NewProfileLink {
    /// The display label for the link.
    pub label: String,
    /// The destination URL of the link.
    pub url: String,
    /// The position used to determine the link's display order.
    pub position: i32,
}

/// Represents a profile link retrieved from the database.
#[derive(Debug, Clone)]
pub struct DatabaseLink {
    /// The unique identifier of the link.
    pub id: i64,
    /// The identifier of the organization the link belongs to.
    pub organization_id: i64,
    /// The display label for the link.
    pub label: String,
    /// The destination URL of the link.
    pub url: String,
    /// The position used to determine the link's display order.
    pub position: i32,
    /// The timestamp when the link was created.
    pub created_at: DateTime<Utc>,
    /// The timestamp when the link was last updated.
    pub updated_at: DateTime<Utc>,
}