use std::sync::Arc;

use crate::auth::password::PasswordService;
use crate::auth::session::{Session, SessionError, SessionManager};
use crate::storage::database::{DatabaseError, MemberRepository};
pub struct Authenticator {
    members: Arc<dyn MemberRepository>,
    session_manager: Arc<SessionManager>,
    passwords: Arc<PasswordService>,
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
    #[error("password hash error")]
    PasswordHash(String),
}

impl Authenticator {
    pub(crate) fn new(members: Arc<dyn MemberRepository>, session_manager: Arc<SessionManager>, passwords: Arc<PasswordService>) -> Self {
        Self { members, session_manager, passwords }
    }

    pub async fn login_member(&self, member_id: i64) -> Result<Session, AuthenticationError> {
        let session = self.session_manager.create(member_id).await?;
        Ok(session)
    }
    pub async fn login(&self, credentials: LoginCredentials) -> Result<Session, AuthenticationError> {
        let member = self.members.find_member_credentials_by_email(&credentials.email).await?.ok_or(AuthenticationError::InvalidCredentials)?;

        if !self.passwords.verify_password(credentials.password, member.password_hash).await.map_err(|error| AuthenticationError::PasswordHash(error.to_string()))? {
            return Err(AuthenticationError::InvalidCredentials);
        }
        // TODO: Invalidate all existing sessions for this member on login.
        // Without session rotation, a stolen session remains valid even after
        // the user logs in from another device, and there's no "logout all devices"
        // capability. Consider tracking sessions per member in Redis and deleting
        // old ones on each new login.
        // See: https://cheatsheetseries.owasp.org/cheatsheets/Session_Management_Cheat_Sheet.html#session-id-change
        // login member using member id and return result
        self.login_member(member.member_id).await
    }
}
