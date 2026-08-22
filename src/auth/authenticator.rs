use std::sync::Arc;

use crate::storage::database::{Database, DatabaseError};
use crate::auth::session::{Session, SessionError, SessionManager};
pub struct Authenticator {
    database: Arc<dyn Database>,
    session_manager: Arc<SessionManager>,
}

pub struct LoginCredentials {
    pub email: String,
    pub password: String,
}

#[derive(Debug, thiserror::Error)]
pub enum AuthenticationError {
    #[error("invalid credentials")]
    InvalidCredentials,

    #[error("database error")]
    Database(#[from] DatabaseError),

    #[error("session error")]
    Session(#[from] SessionError),
}

impl Authenticator {
    pub fn new(database: Arc<dyn Database>, session_manager: Arc<SessionManager>) -> Self {
        Self { database, session_manager }
    }

    pub async fn login_member(&self, member_id: i64, organization_id: i64, ) -> Result<Session, AuthenticationError> {
        let session = self.session_manager.create(member_id, organization_id).await?;
        Ok(session)
    }
    pub async fn login(&self, credentials: LoginCredentials) -> Result<Session, AuthenticationError> {
        let member = self.database.find_member_by_email(&credentials.email)
            .await?
            .ok_or(AuthenticationError::InvalidCredentials)?;

        // TODO: Replace this with real password hash verification.
        if member.password_hash != credentials.password {
            return Err(AuthenticationError::InvalidCredentials);
        }
        // login member using member id and return result
        self.login_member(member.id, member.organization_id).await
    }
}
