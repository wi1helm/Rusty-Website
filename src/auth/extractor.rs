use axum::{
    extract::FromRequestParts,
    http::request::Parts,
    response::{IntoResponse, Redirect, Response},
};

use crate::auth::middleware::AuthenticatedUser;

/// Extracts the authenticated user from the current request.
///
/// `require_auth` middleware is responsible for authenticating the request
/// and resolving the application-level authentication context. Once
/// authentication succeeds, the middleware inserts an `AuthenticatedUser`
/// into the request's extensions.
///
/// This extractor simply retrieves that already-resolved context so handlers
/// don't need to know anything about cookies, Redis, sessions, or the
/// authentication middleware.
///
/// The authenticated user contains the member's identity and the organization
/// context associated with the authenticated session.
///
/// Usage in a handler:
///
///     pub async fn dashboard(
///         CurrentUser(user): CurrentUser,
///     ) -> Response {
///         // use user.member_id
///         // use user.organization.id
///         // use user.organization.name
///     }
pub struct CurrentUser(pub AuthenticatedUser);

/// Allows `CurrentUser` to be used as an Axum handler extractor.
///
/// Axum calls this before the handler runs. We look inside the request
/// extensions for the `AuthenticatedUser` that `require_auth` previously
/// resolved and inserted.
///
/// The extractor does not perform authentication, access the session store,
/// or query the database. Those responsibilities belong to the middleware.
///
/// `S` is the application's router state (`Context` in this application).
/// The extractor does not currently need the state itself, but Axum requires
/// it as part of the `FromRequestParts` interface.
impl<S> FromRequestParts<S> for CurrentUser where S: Send + Sync {
    /// The response returned if the authenticated user cannot be extracted.
    ///
    /// Normally this should not happen on protected routes because the
    /// `require_auth` middleware runs before the handler and inserts the
    /// authenticated user into the request extensions.
    ///
    /// This rejection is still useful as a defensive fallback if someone
    /// accidentally uses `CurrentUser` on a route without the authentication
    /// middleware.
    type Rejection = Response;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        // `require_auth` inserts the resolved `AuthenticatedUser` into the
        // request extensions after successfully validating the session and
        // loading the associated organization.
        //
        // We borrow it here initially because the request owns the extension
        // value.
        let Some(user) = parts.extensions.get::<AuthenticatedUser>() else {
            // If no `AuthenticatedUser` exists, this extractor cannot provide
            // the authenticated context required by the handler.
            //
            // On correctly configured protected routes this should normally
            // be unreachable because `require_auth` runs before the handler.
            return Err(Redirect::to("/login").into_response());
        };

        // Clone the authenticated context out of the request extensions.
        //
        // This allows the handler to own its `CurrentUser` while the request
        // continues through Axum's handler machinery.
        Ok(CurrentUser(user.clone()))
    }
}