// database/postgres.rs
use async_trait::async_trait;
use sqlx::PgPool;
use super::{Database, NewMember, NewOrganization, Organization, DatabaseError};


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
    async fn create_organization(&self, tx: &mut sqlx::Transaction<'_, sqlx::Postgres>, organization: NewOrganization) -> Result<Organization, DatabaseError> {
        let organization = sqlx::query_as!(
            Organization,
            r#"
            INSERT INTO organizations (name)
            VALUES ($1)
            RETURNING id, name
            "#,
            organization.name
            )
            .fetch_one(&mut **tx)
            .await?;

        Ok(organization)
    }


    async fn add_member(&self, tx: &mut sqlx::Transaction<'_, sqlx::Postgres>, organization_id: i64, member: NewMember) -> Result<i64, DatabaseError> {
        let record = sqlx::query!(
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
            .map_err(|err| map_unique_violation(err, DatabaseError::MemberAlreadyExists))?;

        Ok(record.id)
    }
}

#[async_trait]
impl Database for PostgresDatabase {
    async fn create_organization_with_owner(&self, organization: NewOrganization, owner: NewMember) -> Result<(Organization, i64), DatabaseError> {
        let mut tx = self.pool.begin().await?;

        let organization = self.create_organization(&mut tx, organization).await?;
        let owner_id = self.add_member(&mut tx, organization.id, owner).await?;

        tx.commit().await?;

        Ok((organization, owner_id))
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
fn map_unique_violation(err: sqlx::Error, mapped: DatabaseError) -> DatabaseError {
    if let sqlx::Error::Database(db_err) = &err {
        if db_err.code().as_deref() == Some("23505") {
            return mapped;
        }
    }
    DatabaseError::Sqlx(err)
}