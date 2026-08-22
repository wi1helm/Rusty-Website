// database/mod.rs
pub mod postgres;

use async_trait::async_trait;

#[derive(Debug, thiserror::Error)]
pub enum DatabaseError {
    #[error("database error")]
    Sqlx(#[from] sqlx::Error),

    #[error("member already exists")]
    MemberAlreadyExists,
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
}

pub struct NewMember {
    pub email: String,
    pub password_hash: String, //TODO add hashing
    pub role: OrganizationRoles,
}

pub struct Organization {
    pub id: i64,
    pub name: String,
}
pub struct Member {
    pub id: i64,
    pub organization_id: i64,
    pub email: String,
    pub password_hash: String,
}

#[async_trait]
pub trait Database: Send + Sync {

    async fn create_organization_with_owner(&self, organization: NewOrganization, owner: NewMember) -> Result<(Organization, i64), DatabaseError>;
    async fn find_member_by_email(&self, email: &str) -> Result<Option<Member>, DatabaseError>;
}