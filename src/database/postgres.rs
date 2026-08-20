// database/postgres.rs
use async_trait::async_trait;
use sqlx::PgPool;
use super::Database;

#[derive(Clone)]
pub struct PostgresDatabase {
    pool: PgPool,
}

impl PostgresDatabase {
    pub async fn new(url: &str) -> Result<Self, sqlx::Error> {
        let pool = PgPool::connect(url).await?;
        Ok(Self { pool })
    }
}

#[async_trait]
impl Database for PostgresDatabase {
    //TODO
}