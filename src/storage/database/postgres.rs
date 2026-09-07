// database/postgres.rs
use async_trait::async_trait;
use sqlx::PgPool;
use crate::config;
use super::{Database, NewMember, NewOrganization, DatabaseOrganization, DatabaseMember, DatabaseError, DatabaseProfileVersion, DatabaseProfile, DatabaseMemberWithOrganization, NewProfileVersion, PublishProfileError, CreateOrganizationError};


#[derive(Clone)]
pub struct PostgresDatabase {
    pool: PgPool,
}

impl PostgresDatabase {
    pub async fn new(url: &str) -> Result<Self, sqlx::Error> {
        let pool = PgPool::connect(url).await?;

        // Run migrations.
        sqlx::migrate!("./migrations").run(&pool).await?;

        Ok(Self { pool })
    }
    /// Creates the organization and returns its database ID.
    ///
    /// This is kept as a small transaction helper because organization creation
    /// is part of a larger operation. The caller is responsible for committing
    /// the transaction only after the owner and profile have also been created.
    async fn create_organization(&self, tx: &mut sqlx::Transaction<'_, sqlx::Postgres>, organization: NewOrganization) -> Result<i64, CreateOrganizationError> {
        let organization = sqlx::query_as!(
            DatabaseOrganization,
            r#"
            INSERT INTO organizations (name, slug)
            VALUES ($1, $2)
            RETURNING id, name, slug
            "#,
            organization.name,
            organization.slug
            )
            .fetch_one(&mut **tx)
            .await
            .map_err(|err| map_unique_violation(err, CreateOrganizationError::OrganizationSlugAlreadyExists))?;

        Ok(organization.id)
    }
    /// Adds a member to an existing organization and returns the member ID.
    ///
    /// The member is created inside the same transaction as the organization,
    /// so a failed signup cannot leave an organization without its owner.
    async fn add_member(&self, tx: &mut sqlx::Transaction<'_, sqlx::Postgres>, organization_id: i64, member: NewMember) -> Result<i64, CreateOrganizationError> {
        let member = sqlx::query!(
            r#"
            INSERT INTO members (organization_id, email, password_hash, role)
            VALUES ($1, $2, $3, $4)
            RETURNING id
            "#,
            organization_id,
            member.email,
            member.password_hash,
            member.role.as_str()
            )
            .fetch_one(&mut **tx)
            .await
            .map_err(|err| map_unique_violation(err, CreateOrganizationError::MemberAlreadyExists))?;

        Ok(member.id)
    }
    /// Creates the profile belonging to an organization and returns its ID.
    ///
    /// Every organization is expected to have exactly one profile. The UNIQUE
    /// constraint on profiles.organization_id enforces this at the database level.
    async fn create_profile(&self, tx: &mut sqlx::Transaction<'_, sqlx::Postgres>, organization_id: i64, ) -> Result<i64, DatabaseError> {
        let profile = sqlx::query!(
            r#"
            INSERT INTO profiles (organization_id)
            VALUES ($1)
            RETURNING id
            "#,
            organization_id
            )
            .fetch_one(&mut **tx)
            .await?;

        Ok(profile.id)
    }
    /// Creates an immutable snapshot of a profile at a specific version.
    ///
    /// Profile content is stored in profile_versions rather than directly on
    /// profiles. Each new change creates another version instead of modifying an
    /// existing version. The caller is responsible for choosing the version
    /// number and updating the profile's current/published pointers.
    async fn create_profile_version(&self, tx: &mut sqlx::Transaction<'_, sqlx::Postgres>, profile_id: i64, version: i32, display_name: String, about: String, created_by_member_id: i64) -> Result<i64, DatabaseError> {
        let profile_version = sqlx::query!(
            r#"
            INSERT INTO profile_versions (
            profile_id,
            version,
            display_name,
            about,
            created_by_member_id
            )
            VALUES ($1, $2, $3, $4, $5)
            RETURNING id
            "#,
            profile_id,
            version,
            display_name,
            about,
            created_by_member_id
            )
            .fetch_one(&mut **tx)
            .await?;

        Ok(profile_version.id)
    }

    async fn set_current_version(&self, tx: &mut sqlx::Transaction<'_, sqlx::Postgres>, profile_id: i64, version_id: i64) -> Result<(), DatabaseError> {
        sqlx::query!(
        r#"
        UPDATE profiles
        SET
            current_version_id = $2,
            updated_at = NOW()
        WHERE id = $1
        "#,
        profile_id,
        version_id
        )
        .execute(&mut **tx)
        .await?;

        Ok(())
    }
    async fn set_published_version(&self, tx: &mut sqlx::Transaction<'_, sqlx::Postgres>, profile_id: i64, version_id: i64) -> Result<(), DatabaseError> {
        sqlx::query!(
        r#"
        UPDATE profiles
        SET
            published_version_id = $2,
            updated_at = NOW()
        WHERE id = $1
        "#,
        profile_id,
        version_id
        )
        .execute(&mut **tx)
        .await?;

        Ok(())
    }
}

#[async_trait]
impl Database for PostgresDatabase {
    async fn create_organization_with_owner_and_profile(&self, organization: NewOrganization, owner: NewMember) -> Result<(i64, i64), CreateOrganizationError> {
        let mut tx = self.pool.begin().await?;
        // Clone name before losing ownership to organization var.
        let organization_name = organization.name.clone();

        let organization_id = self.create_organization(&mut tx, organization).await?;
        let owner_id = self.add_member(&mut tx, organization_id, owner).await?;
        let profile_id = self.create_profile(&mut tx, organization_id).await?;
        let version_id = self.create_profile_version(&mut tx, profile_id, config::INITIAL_PROFILE_VERSION, organization_name, String::new(), owner_id).await?;
        // The initial version is both the current version and
        // the version visible to the public.
        self.set_current_version(&mut tx, profile_id, version_id).await?;
        self.set_published_version(&mut tx, profile_id, version_id).await?;
        tx.commit().await?;

        Ok((organization_id, owner_id))
    }

    async fn create_new_profile_version(&self, profile_version: NewProfileVersion) -> Result<i64, DatabaseError> {
        let mut tx = self.pool.begin().await?;

        let version_id = self.create_profile_version(
            &mut tx,
            profile_version.profile_id,
            profile_version.version,
            profile_version.display_name,
            profile_version.about,
            profile_version.created_by_member_id,
        ).await?;

        self.set_current_version(&mut tx, profile_version.profile_id, version_id).await?;

        tx.commit().await?;

        Ok(version_id)
    }

    async fn find_member_by_email(&self, email: &str) -> Result<Option<DatabaseMember>, DatabaseError> {
        let member = sqlx::query_as!(
            DatabaseMember,
            r#"
            SELECT id, organization_id, email, password_hash
            FROM members
            WHERE email = $1
            "#,
            email
            )
            .fetch_optional(&self.pool)
            .await?;

        Ok(member)
    }
    //TODO think about orgnazition view structs meaning that we dont use the Orgnizatin struct everywhere but sub views of if depending on what info we need
    // as of now no problem due to small abount of data
    async fn find_organization_by_organization_id(&self, organization_id: i64) -> Result<Option<DatabaseOrganization>, DatabaseError> {
        let organization = sqlx::query_as!(
            DatabaseOrganization,
            r#"
            SELECT id, name, slug
            FROM organizations
            WHERE id = $1
            "#,
            organization_id
            )
            .fetch_optional(&self.pool)
            .await?;

        Ok(organization)
    }

    async fn find_organization_by_slug(&self, slug: &str) -> Result<Option<DatabaseOrganization>, DatabaseError> {
        let organization = sqlx::query_as!(
            DatabaseOrganization,
            r#"
            SELECT id, name, slug
            FROM organizations
            WHERE slug = $1
            "#,
            slug
            )
            .fetch_optional(&self.pool)
            .await?;

        Ok(organization)
    }

    /// Finds the profile belonging to an organization.
    ///
    /// An organization is expected to have exactly one profile. The UNIQUE
    /// constraint on profiles.organization_id enforces this relationship at the
    /// database level, while Option is used because the profile may be missing
    /// if the database is in an inconsistent state.
    async fn find_profile_by_organization_id(&self, organization_id: i64) -> Result<Option<DatabaseProfile>, DatabaseError> {
        let profile = sqlx::query_as!(
            DatabaseProfile,
            r#"
            SELECT
            id,
            organization_id,
            current_version_id,
            published_version_id
            FROM profiles
            WHERE organization_id = $1
            "#,
           organization_id
            )
            .fetch_optional(&self.pool)
            .await?;

        Ok(profile)
    }
    /// Finds a member together with their organization in a single query.
    ///
    /// Members are required to belong to an organization by the database schema,
    /// so an INNER JOIN guarantees that a returned result represents a valid
    /// member-organization relationship. A missing result means the member no
    /// longer exists (for example, due to the organization being deleted), which
    /// may indicate a stale authentication session.
    async fn find_member_with_organization_by_member_id(&self, member_id: i64) -> Result<Option<DatabaseMemberWithOrganization>, DatabaseError> {
        let row = sqlx::query!(
            r#"
            SELECT
                m.id AS member_id,
                m.organization_id AS member_organization_id,
                m.email AS member_email,
                m.password_hash AS member_password_hash,

                o.id AS organization_id,
                o.name AS organization_name,
                o.slug AS organization_slug
            FROM members m
            INNER JOIN organizations o
                ON o.id = m.organization_id
            WHERE m.id = $1
            "#,
            member_id
            )
            .fetch_optional(&self.pool)
            .await?;

        Ok(row.map(|row| DatabaseMemberWithOrganization {
            member: DatabaseMember {
                id: row.member_id,
                organization_id: row.member_organization_id,
                email: row.member_email,
                password_hash: row.member_password_hash,
            },
            organization: DatabaseOrganization {
                id: row.organization_id,
                name: row.organization_name,
                slug: row.organization_slug,
            },
        }))
    }
    /// Finds a specific profile version by its database ID.
    ///
    /// Profile versions are immutable snapshots of profile content. The ID is
    /// obtained from either current_version_id or published_version_id on the
    /// associated DatabaseProfile.
    async fn find_profile_version_by_version_id(&self, version_id: i64) -> Result<Option<DatabaseProfileVersion>, DatabaseError> {
        let profile_version = sqlx::query_as!(
            DatabaseProfileVersion,
            r#"
            SELECT
            id,
            profile_id,
            version,
            display_name,
            about,
            created_by_member_id
            FROM profile_versions
            WHERE id = $1
            "#,
            version_id
            )
            .fetch_optional(&self.pool)
            .await?;

        Ok(profile_version)
    }

    async fn publish_current_profile_version(&self, profile_id: i64) -> Result<(), PublishProfileError> {
        let mut tx = self.pool.begin().await?;

        let profile = sqlx::query!(
            r#"
            SELECT current_version_id
            FROM profiles
            WHERE id = $1
            "#,
            profile_id
            )
            .fetch_optional(&mut *tx)
            .await?;

        let Some(profile) = profile else {
            return Err(PublishProfileError::ProfileNotFound);
        };

        let Some(current_version_id) = profile.current_version_id else {
            return Err(PublishProfileError::ProfileHasNoCurrentVersion);
        };

        self.set_published_version(&mut tx, profile_id, current_version_id).await?;

        tx.commit().await?;

        Ok(())
    }
}





/// Maps a raw sqlx error into a `DatabaseError`.
///
/// If the error came back from Postgres itself (as opposed to a connection
/// or network failure) and its SQLSTATE code is `23505` ("unique_violation"),
/// we know a UNIQUE constraint was hit — e.g. a duplicate email or duplicate
/// organization name. In that case we return the caller-supplied `mapped`
/// variant instead, so upstream code (like the signup handler) can show a
/// specific "already exists" message rather than a generic server error.
///
/// Any other kind of error (network issue, syntax error, etc.) falls through
/// unchanged, wrapped in `DatabaseError::Sqlx`.
///TODO check so postgres error code for unique_violation is consistent otherwise this code would fail in future. or make better code
fn map_unique_violation(err: sqlx::Error, mapped: CreateOrganizationError) -> CreateOrganizationError {
    if let sqlx::Error::Database(db_err) = &err {
        if db_err.code().as_deref() == Some("23505") {
            return mapped;
        }
    }
    CreateOrganizationError::Database(DatabaseError::Sqlx(err))
}