use chrono::{DateTime, Utc};

use crate::storage::database::{DatabaseError, DatabaseMember, DatabaseMemberWithOrganization, DatabaseOrganization, MemberCredentials, OrganizationRole};

pub struct MemberCredentialsRow {
    pub member_id: i64,
    pub password_hash: String,
}

pub struct MemberWithOrganizationRow {
    pub member_id: i64,
    pub member_organization_id: i64,
    pub member_email: String,
    pub member_role: String,
    pub member_created_at: DateTime<Utc>,
    pub member_updated_at: DateTime<Utc>,

    pub organization_id: i64,
    pub organization_name: String,
    pub organization_slug: String,
    pub organization_created_at: DateTime<Utc>,
}

pub fn map_member_credentials(row: MemberCredentialsRow) -> MemberCredentials {
    MemberCredentials { member_id: row.member_id, password_hash: row.password_hash }
}

pub fn map_member_with_organization(row: MemberWithOrganizationRow) -> Result<DatabaseMemberWithOrganization, DatabaseError> {
    let role = OrganizationRole::from_str(&row.member_role).ok_or_else(|| DatabaseError::InvalidOrganizationRole(row.member_role))?;

    Ok(DatabaseMemberWithOrganization {
        member: DatabaseMember { id: row.member_id, organization_id: row.member_organization_id, email: row.member_email, role, created_at: row.member_created_at, updated_at: row.member_updated_at },
        organization: DatabaseOrganization { id: row.organization_id, name: row.organization_name, slug: row.organization_slug, created_at: row.organization_created_at },
    })
}
