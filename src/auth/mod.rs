mod authenticator;
pub(crate) mod session;
pub(crate) mod middleware;
pub(crate) mod extractor;
pub use extractor::AuthenticatedSession;

pub use authenticator::{
    Authenticator,
    AuthenticationError,
    LoginCredentials,
};
