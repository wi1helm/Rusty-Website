use axum::{
    extract::FromRequestParts,
    http::request::Parts,
};
use axum::response::IntoResponse;

use crate::auth::session::Session;

/// Extracts the authenticated session from the current request.
///
/// `require_auth` middleware is responsible for actually authenticating the
/// request. Once authentication succeeds, the middleware inserts the
/// `Session` into the request's extensions.
///
/// This extractor simply retrieves that already-authenticated session so
/// handlers don't need to know anything about cookies, Redis, or the
/// authentication middleware.
///
/// Usage in a handler:
///
///     pub async fn dashboard(
///         AuthenticatedSession(session): AuthenticatedSession,
///     ) -> HTML<String> {
///         // use session.member_id, session.organization_id, etc.
///     }
pub struct AuthenticatedSession(pub Session);

/// Allows `AuthenticatedSession` to be used as an Axum handler extractor.
///
/// Axum calls this before the handler runs. We look inside the request
/// extensions for the `Session` that `require_auth` previously inserted.
///
/// `S` is the application's router state (`Context` in this application).
/// The extractor does not currently need the state itself, but Axum requires
/// it as part of the `FromRequestParts` interface.
impl<S> FromRequestParts<S> for AuthenticatedSession where S: Send + Sync {
    /// The response returned if the session cannot be extracted.
    ///
    /// Normally this should not happen on protected routes because the
    /// `require_auth` middleware runs first and inserts the session.
    ///
    /// This rejection is still useful as a defensive fallback if someone
    /// accidentally uses `AuthenticatedSession` on a route without the
    /// authentication middleware.
    type Rejection = axum::response::Response;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        // `require_auth` inserts the authenticated Session into the request
        // extensions after successfully validating the session cookie.
        //
        // We only borrow it here initially because the request owns the
        // extension value.
        let Some(session) = parts.extensions.get::<Session>() else {
            // If no Session exists, this extractor cannot provide an
            // AuthenticatedSession to the handler.
            //
            // On correctly configured protected routes this should normally
            // be unreachable because `require_auth` runs before the handler.
            return Err(axum::response::Redirect::to("/login").into_response());
        };

        // Clone the Session out of the request extensions.
        //
        // This allows the handler to own its AuthenticatedSession while the
        // request continues through Axum's handler machinery.
        Ok(AuthenticatedSession(session.clone()))
    }
}
