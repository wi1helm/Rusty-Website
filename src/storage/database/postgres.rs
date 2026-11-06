use async_trait::async_trait;
use sqlx::{PgPool, Transaction};
use chrono::{DateTime, Utc};
use crate::storage::object::defaults::{default_banner, default_logo};
use super::{Database, NewMember, NewOrganization, DatabaseOrganization, DatabaseMember, DatabaseError, DatabaseProfile, DatabaseMemberWithOrganization, CreateOrganizationError, NewProfileImage, DatabaseImage, ProfileImageType, DatabaseLink, NewProfileLink, NewProfile, OrganizationRole, UpdateProfile};
use super::mapping::member::{
    map_member,
    map_member_with_organization,
    MemberRow,
    MemberWithOrganizationRow,
};

#[derive(Clone)]
pub struct PostgresDatabase {
    pool: PgPool,
}

impl PostgresDatabase {
    /// Creates a new PostgreSQL database connection pool and runs pending migrations.
    pub async fn new(url: &str) -> Result<Self, sqlx::Error> {
        let pool = PgPool::connect(url).await?;

        // Run migrations.
        sqlx::migrate!("./migrations").run(&pool).await?;

        Ok(Self { pool })
    }

    /// Creates a new organization and returns its database ID.
    async fn create_organization(&self, tx: &mut Transaction<'_, sqlx::Postgres>, organization: NewOrganization) -> Result<i64, CreateOrganizationError> {
        let result = sqlx::query_scalar!(
            r#"
            INSERT INTO organizations (name, slug)
            VALUES ($1, $2)
            RETURNING id
            "#,
            organization.name,
            organization.slug,
        )
            .fetch_one(&mut **tx)
            .await;

        result.map_err(|err| { map_unique_violation(err, CreateOrganizationError::OrganizationSlugAlreadyExists, ) })
    }

    /// Creates a member in an existing organization and returns the member ID.
    async fn create_member(&self, tx: &mut Transaction<'_, sqlx::Postgres>, organization_id: i64, member: NewMember) -> Result<i64, CreateOrganizationError> {
        let result = sqlx::query_scalar!(
            r#"
            INSERT INTO organization_members (
                organization_id,
                email,
                password_hash,
                role
            )
            VALUES ($1, $2, $3, $4)
            RETURNING id
            "#,
            organization_id,
            member.email,
            member.password_hash,
            member.role.as_str(),
        )
            .fetch_one(&mut **tx)
            .await;

        result.map_err(|err| { map_unique_violation(err, CreateOrganizationError::MemberAlreadyExists) })
    }

    /// Creates the profile belonging to an organization and returns its database ID.
    async fn create_profile(&self, tx: &mut sqlx::Transaction<'_, sqlx::Postgres>, organization_id: i64, member_id: i64, profile: NewProfile) -> Result<i64, DatabaseError> {
        let display_name = profile.display_name;

        let profile_id = sqlx::query_scalar!(
            r#"
            INSERT INTO organization_profiles (
                organization_id,
                display_name,
                updated_by_member_id
            )
            VALUES ($1, $2, $3)
            RETURNING id
            "#,
            organization_id,
            display_name,
            member_id,
        )
            .fetch_one(&mut **tx)
            .await?;

        let logo = default_logo(organization_id).map_err(|_| DatabaseError::InvalidDefaultProfileImage)?;
        let banner = default_banner(organization_id).map_err(|_| DatabaseError::InvalidDefaultProfileImage)?;

        self.create_profile_image(tx, logo).await?;
        self.create_profile_image(tx, banner).await?;
        Ok(profile_id)
    }

    async fn create_profile_image(&self, tx: &mut Transaction<'_, sqlx::Postgres>, image: NewProfileImage) -> Result<(), DatabaseError> {
        sqlx::query!(
            r#"
            INSERT INTO organization_profile_images (
                organization_id,
                image_type,
                storage_key,
                content_type,
                width,
                height,
                file_size
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            "#,
            image.organization_id,
            image.image_type.as_str(),
            image.storage_key,
            image.content_type,
            image.width,
            image.height,
            image.file_size,
        )
            .execute(&mut **tx)
            .await?;

        Ok(())
    }
}
#[async_trait]
impl Database for PostgresDatabase {
    /// Creates an organization, assigns its owner, and creates its default profile in a single transaction.
    async fn create_organization_with_owner_and_profile(&self, organization: NewOrganization, owner: NewMember, ) -> Result<(i64, i64), CreateOrganizationError> {
        let mut tx = self.pool.begin().await?;

        let profile = NewProfile {
            display_name: organization.name.clone(),
        };

        let member = NewMember {
            email: owner.email,
            password_hash: owner.password_hash,
            role: OrganizationRole::Owner,
        };

        let organization_id = self.create_organization(&mut tx, organization).await?;

        let owner_id = self.create_member(&mut tx, organization_id, member).await?;
        // Just let it void itself but make it exist here so we can use later.
        let _profile_id = self.create_profile(&mut tx, organization_id, owner_id, profile).await?;

        tx.commit().await?;

        Ok((organization_id, owner_id))
    }

    /// Finds an organization by its database ID.
    async fn find_organization_by_organization_id(&self, organization_id: i64) -> Result<Option<DatabaseOrganization>, DatabaseError> {
        let organization = sqlx::query_as!(
            DatabaseOrganization,
            r#"
            SELECT
                id,
                name,
                slug,
                created_at AS "created_at: DateTime<Utc>"
            FROM organizations
            WHERE id = $1
            "#,
            organization_id,
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(organization)
    }

    /// Finds an organization by its unique slug.
    async fn find_organization_by_slug(&self, slug: &str) -> Result<Option<DatabaseOrganization>, DatabaseError> {
        let organization = sqlx::query_as!(
            DatabaseOrganization,
            r#"
            SELECT
                id,
                name,
                slug,
                created_at AS "created_at: DateTime<Utc>"
            FROM organizations
            WHERE slug = $1
            "#,
            slug,
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(organization)
    }

    /// Finds a member by their email address.
    async fn find_member_by_email(&self, email: &str) -> Result<Option<DatabaseMember>, DatabaseError> {
        let Some(row) = sqlx::query_as!(
            MemberRow,
            r#"
            SELECT
                id,
                organization_id,
                email,
                password_hash,
                role,
                created_at,
                updated_at
            FROM organization_members
            WHERE email = $1
            "#,
            email,
            ).fetch_optional(&self.pool).await?
        else { return Ok(None);};

        Ok(Some(map_member(row)?))
    }

    /// Finds a member by their ID, including their organization.
    async fn find_member_with_organization_by_member_id(&self, member_id: i64) -> Result<Option<DatabaseMemberWithOrganization>, DatabaseError> {
        let Some(row) = sqlx::query_as!(
            MemberWithOrganizationRow,
            r#"
            SELECT
                m.id AS member_id,
                m.organization_id AS member_organization_id,
                m.email AS member_email,
                m.password_hash AS member_password_hash,
                m.role AS member_role,
                m.created_at AS member_created_at,
                m.updated_at AS member_updated_at,

                o.id AS organization_id,
                o.name AS organization_name,
                o.slug AS organization_slug,
                o.created_at AS organization_created_at
            FROM organization_members m
            INNER JOIN organizations o
                ON o.id = m.organization_id
            WHERE m.id = $1
            "#,
            member_id,
        ).fetch_optional(&self.pool).await?
        else { return Ok(None); };

        Ok(Some(map_member_with_organization(row)?))
    }

    /// Finds the profile associated with an organization.
    async fn find_profile_by_organization_id(&self, organization_id: i64) -> Result<Option<DatabaseProfile>, DatabaseError> {
        let Some(profile) = sqlx::query!(
            r#"
            SELECT
                id,
                organization_id,
                display_name,
                about,
                updated_by_member_id,
                created_at AS "created_at: DateTime<Utc>",
                updated_at AS "updated_at: DateTime<Utc>"
            FROM organization_profiles
            WHERE organization_id = $1
            "#,
            organization_id,
        )
        .fetch_optional(&self.pool)
        .await?

        else { return Ok(None);};

        let images = sqlx::query!(
            r#"
            SELECT
                id,
                organization_id,
                image_type,
                storage_key,
                content_type,
                width,
                height,
                file_size,
                created_at AS "created_at: DateTime<Utc>",
                updated_at AS "updated_at: DateTime<Utc>"
            FROM organization_profile_images
            WHERE organization_id = $1
            "#,
            organization_id,
        )
        .fetch_all(&self.pool)
        .await?;

        let mut logo: Option<DatabaseImage> = None;
        let mut banner: Option<DatabaseImage> = None;

        for image in images {
            let image_type = ProfileImageType::from_str(&image.image_type).ok_or_else(|| { DatabaseError::InvalidProfileImageType(image.image_type.clone()) })?;
            let database_image = DatabaseImage {
                id: image.id,
                organization_id: image.organization_id,
                image_type,
                storage_key: image.storage_key,
                content_type: image.content_type,
                width: image.width,
                height: image.height,
                file_size: image.file_size,
                created_at: image.created_at,
                updated_at: image.updated_at,
            };

            match image_type {
                ProfileImageType::Logo => logo = Some(database_image),
                ProfileImageType::Banner => banner = Some(database_image),
            }
        }

        let logo = logo.ok_or_else(|| {
            DatabaseError::Other(format!(
                "Missing profile logo for organization {organization_id}"
            ))
        })?;

        let banner = banner.ok_or_else(|| {
            DatabaseError::Other(format!(
                "Missing profile banner for organization {organization_id}"
            ))
        })?;

        let links = sqlx::query_as!(
            DatabaseLink,
            r#"
            SELECT
                id,
                organization_id,
                label,
                url,
                position,
                created_at AS "created_at: DateTime<Utc>",
                updated_at AS "updated_at: DateTime<Utc>"
            FROM organization_profile_links
            WHERE organization_id = $1
            ORDER BY position ASC
            "#,
            organization_id,
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(Some(DatabaseProfile {
            id: profile.id,
            organization_id: profile.organization_id,
            display_name: profile.display_name,
            about: profile.about,
            logo,
            banner,
            links,
            updated_by_member_id: profile.updated_by_member_id,
            created_at: profile.created_at,
            updated_at: profile.updated_at,
        }))
    }

    /// Updates the profile associated with an organization.
    async fn update_profile(&self, organization_id: i64, member_id: i64, profile: UpdateProfile) -> Result<Option<DatabaseProfile>, DatabaseError> {
        let updated_profile_id = sqlx::query_scalar!(
            r#"
            UPDATE organization_profiles
            SET
                display_name = $2,
                about = $3,
                updated_by_member_id = $4,
                updated_at = NOW()
            WHERE organization_id = $1
            RETURNING id
            "#,
            organization_id,
            profile.display_name,
            profile.about,
            member_id,
        ).fetch_optional(&self.pool).await?;

        let Some(_) = updated_profile_id else {
            return Ok(None);
        };

        self.find_profile_by_organization_id(organization_id).await
    }

    /// Updates an organization's profile image.
    async fn update_profile_image(&self, image: NewProfileImage) -> Result<Option<DatabaseImage>, DatabaseError> {
        let image_type = image.image_type.as_str();

        let Some(image) = sqlx::query!(
            r#"
            INSERT INTO organization_profile_images (
                organization_id,
                image_type,
                storage_key,
                content_type,
                width,
                height,
                file_size
            )
            SELECT
                $1, $2, $3, $4, $5, $6, $7
            FROM organization_profiles
            WHERE organization_id = $1
            ON CONFLICT (organization_id, image_type)
            DO UPDATE SET
                storage_key = EXCLUDED.storage_key,
                content_type = EXCLUDED.content_type,
                width = EXCLUDED.width,
                height = EXCLUDED.height,
                file_size = EXCLUDED.file_size,
                updated_at = NOW()
            RETURNING
                id,
                organization_id,
                image_type,
                storage_key,
                content_type,
                width,
                height,
                file_size,
                created_at AS "created_at: DateTime<Utc>",
                updated_at AS "updated_at: DateTime<Utc>"
            "#,
            image.organization_id,
            image_type,
            image.storage_key,
            image.content_type,
            image.width,
            image.height,
            image.file_size,
        ).fetch_optional(&self.pool).await?
        else { return Ok(None);};

        let image_type = ProfileImageType::from_str(&image.image_type)
            .ok_or_else(|| DatabaseError::InvalidProfileImageType(image.image_type))?;

        Ok(Some(DatabaseImage {
            id: image.id,
            organization_id: image.organization_id,
            image_type,
            storage_key: image.storage_key,
            content_type: image.content_type,
            width: image.width,
            height: image.height,
            file_size: image.file_size,
            created_at: image.created_at,
            updated_at: image.updated_at,
        }))
    }

    /// Updates the profile links associated with an organization.
    async fn update_profile_links(&self, organization_id: i64, links: Vec<NewProfileLink>, ) -> Result<Vec<DatabaseLink>, DatabaseError> {
        let mut tx = self.pool.begin().await?;

        // Make sure the organization has a profile.
        // Lock the profile row while replacing its links.
        let profile_id = sqlx::query_scalar!(
            r#"
            SELECT id
            FROM organization_profiles
            WHERE organization_id = $1
            FOR UPDATE
            "#,
            organization_id,
        ).fetch_optional(&mut *tx).await?;

        let Some(_) = profile_id else { return Ok(Vec::new()); };

        // Remove the existing links.
        sqlx::query!(
            r#"
            DELETE FROM organization_profile_links
            WHERE organization_id = $1
            "#,
            organization_id,
        ).execute(&mut *tx).await?;

        // Insert the replacement links.
        for link in links {
            sqlx::query!(
                r#"
                INSERT INTO organization_profile_links (
                    organization_id,
                    label,
                    url,
                    position
                )
                VALUES ($1, $2, $3, $4)
                "#,
                organization_id,
                link.label,
                link.url,
                link.position,
            ).execute(&mut *tx).await?;
        }

        // Load the resulting links in display order.
        let links = sqlx::query_as!(
            DatabaseLink,
            r#"
            SELECT
                id,
                organization_id,
                label,
                url,
                position,
                created_at AS "created_at: DateTime<Utc>",
                updated_at AS "updated_at: DateTime<Utc>"
            FROM organization_profile_links
            WHERE organization_id = $1
            ORDER BY position ASC
            "#,
            organization_id,
        ).fetch_all(&mut *tx).await?;

        tx.commit().await?;

        Ok(links)
    }
}

/// Maps a PostgreSQL unique-constraint violation to a specific organization-creation error.
fn map_unique_violation(err: sqlx::Error, mapped: CreateOrganizationError) -> CreateOrganizationError {
    if let sqlx::Error::Database(db_err) = &err {
        if db_err.code().as_deref() == Some("23505") {
            return mapped;
        }
    }
    CreateOrganizationError::Database(DatabaseError::Sqlx(err))
}