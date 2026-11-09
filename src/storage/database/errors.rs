// database/errors.rs

#[derive(Debug, thiserror::Error)]
pub enum DatabaseError {
    #[error("database error")]
    Sqlx(#[from] sqlx::Error),

    #[error("invalid organization role in database: {0}")]
    InvalidOrganizationRole(String),

    #[error("invalid profile image type in database: {0}")]
    InvalidProfileImageType(String),

    #[error("failed to create database profile images")]
    InvalidDefaultProfileImage,

    #[error("invalid position status in database: {0}")]
    InvalidPositionStatus(String),
    
    #[error("database error")]
    Other(String),
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
        Self::Database(DatabaseError::Sqlx(error))
    }
}