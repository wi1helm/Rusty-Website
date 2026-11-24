use std::sync::Arc;

use crate::auth::session::{Session, SessionError, SessionManager};
use crate::storage::database::{Database, DatabaseError};
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
        Self {
            database,
            session_manager,
        }
    }

    pub async fn login_member(&self, member_id: i64) -> Result<Session, AuthenticationError> {
        let session = self.session_manager.create(member_id).await?;
        Ok(session)
    }
    pub async fn login(&self, credentials: LoginCredentials, ) -> Result<Session, AuthenticationError> {
        let member = self.database.find_member_by_email(&credentials.email).await?.ok_or(AuthenticationError::InvalidCredentials)?;

        // TODO: Replace this with real password hash verification.
        if member.password_hash != credentials.password { return Err(AuthenticationError::InvalidCredentials); }
        // TODO: Invalidate all existing sessions for this member on login.
        // Without session rotation, a stolen session remains valid even after
        // the user logs in from another device, and there's no "logout all devices"
        // capability. Consider tracking sessions per member in Redis and deleting
        // old ones on each new login.
        // See: https://cheatsheetseries.owasp.org/cheatsheets/Session_Management_Cheat_Sheet.html#session-id-change
        // login member using member id and return result
        self.login_member(member.id).await
    }
}
