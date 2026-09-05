// auth/mod.rs

mod authenticator;
pub(crate) mod session;
pub(crate) mod middleware;
mod extractor;

pub use authenticator::{
    Authenticator,
    AuthenticationError,
    LoginCredentials,
};

pub use extractor::CurrentUser;
/// Represents the authenticated identity and organization context
/// associated with the current request.
///
/// This is intentionally a small request-level context rather than a
/// full member or organization record. Additional data such as profile
/// information, settings, or organization members should be loaded
/// separately when needed.
#[derive(Debug, Clone)]
pub struct AuthenticatedUser {
    pub member_id: i64,
    pub organization: AssociatedOrganization,

}

/// Represents the organization associated with an authenticated member.
///
/// NOTE:
/// Only commonly required organization information is included here.
/// More detailed organization data should be loaded separately when needed.
#[derive(Debug, Clone)]
pub struct AssociatedOrganization {
    pub id: i64,
    pub name: String,
}
