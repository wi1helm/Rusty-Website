use axum::{
    extract::{Request, State},
    middleware::Next,
    response::{IntoResponse, Redirect, Response},
};
use axum_extra::extract::cookie::CookieJar;
use crate::auth::{AssociatedOrganization, AuthenticatedUser};
use crate::auth::session::SessionError;
use crate::Context;

/// Authentication middleware for protected routes.
///
/// The browser only sends the session ID in the `session` cookie. The actual
/// session data remains server-side in the session store (Redis as of writing).
///
/// The middleware is responsible for:
/// - reading the session cookie
/// - looking up the session in the session store
/// - rejecting requests that are not authenticated
/// - resolving the organization associated with the authenticated session
/// - constructing the application-level `AuthenticatedUser` context
/// - putting the authenticated user context into request extensions
/// - passing authenticated requests to the actual route handler
///
/// This keeps authentication and session-store details out of individual
/// protected handlers.
///
/// A handler such as `dashboard` can simply extract `CurrentUser` and access
/// the authenticated member and organization without knowing anything about
/// cookies, Redis, or the underlying `Session` type.
///
/// Note: this middleware should only be put on protected routes.
pub async fn require_auth(State(context): State<Context>, jar: CookieJar, mut request: Request, next: Next) -> Response {
    //TODO(security): Consider clearing the session cookie when the session is
    // missing or invalid. This avoids repeatedly sending a stale session ID.
    // This will require returning a response with an updated CookieJar rather
    // than only returning a Redirect.
    //
    //TODO(security): Consider whether an expired/invalid session should be
    // distinguishable from a missing session cookie for logging/metrics.
    // The user-facing response should remain the same so we don't reveal
    // unnecessary authentication details.
    //
    //TODO: Consider adding explicit logging/metrics for authentication
    // failures once application observability is in place. In particular,
    // distinguish missing sessions from cache failures and serialization
    // failures.

    // The session cookie contains the ID of the server-side session.
    //
    // The cookie itself does not contain the authenticated member or
    // organization information. It only gives us the identifier we need to
    // look up the server-side session.
    //
    // No cookie means there is no session ID that we can use to authenticate
    // the request, so send the user to the login page.
    let Some(cookie) = jar.get("session") else {
        return Redirect::to("/login").into_response();
    };

    let session_id = cookie.value();

    // Look up the session using the ID supplied by the browser.
    //
    // The session itself is stored server-side. A successfully retrieved
    // session establishes which member and organization this request belongs
    // to.
    let session = match context.session_manager.get(session_id).await {
        // The session exists and was successfully loaded.
        //
        // Authentication has succeeded. We continue below and use the session
        // to construct the application-level authenticated user context.
        Ok(Some(session)) => session,

        // The session store lookup succeeded, but there is no session with
        // this ID.
        //
        // This can happen when:
        // - the session expired
        // - the user logged out
        // - the session was deleted
        // - the cookie contains an invalid or old session ID
        //
        // This is not a server error. From the application's perspective,
        // the request simply isn't authenticated anymore.
        Ok(None) => {
            return Redirect::to("/login").into_response();
        }

        // The session store itself could not be accessed.
        //
        // This is different from `Ok(None)`: we don't know whether the user
        // actually has a valid session because the server failed to check.
        //
        // Cache errors are generally expected to be transient infrastructure
        // problems, such as Redis being unavailable or a connection timing
        // out.
        Err(SessionError::Cache(error)) => {
            tracing::warn!("session cache error during authentication: {error}");

            return Redirect::to("/login?error=server-error").into_response();
        }

        // The session data existed, but we could not deserialize it.
        //
        // This is more serious than a normal cache failure. It could indicate
        // corrupted data or that the format of stored sessions has changed
        // without handling old sessions.
        Err(SessionError::Serialization(error)) => {
            // The session ID is effectively a bearer credential. Someone who
            // obtains a valid session ID may be able to authenticate as that
            // user.
            //
            // IMPORTANT: Do not log the session ID. Application logs could
            // potentially be accessed by people or systems that should not
            // have access to authentication credentials.
            tracing::error!(
                "session serialization error during authentication: {error}"
            );

            return Redirect::to("/login?error=server-error").into_response();
        }
    };

    // The session is valid, so authentication has succeeded.
    //
    // The session is an internal authentication/session-store representation.
    // It is intentionally not exposed to downstream handlers.
    //
    // Instead, resolve the organization referenced by the session and create
    // the application-level authentication context that handlers actually
    // need.
    let member_with_organization = match context.database.find_member_with_organization_by_member_id(session.member_id).await {
        // The member and its organization both exist.
        //
        // The INNER JOIN used by the database query guarantees that this
        // represents a valid member-organization relationship.
        Ok(Some(member_with_organization)) => member_with_organization,

        // The session references a member that no longer exists.
        //
        // This can happen when the member was deleted, for example as a result
        // of their organization being deleted. The session is therefore stale
        // and cannot be used to construct an authenticated request context.
        Ok(None) => {
            //TODO remove stale cookie and stale redis value as this session id is invalid
            tracing::warn!(member_id = session.member_id,"authenticated session references a missing member");
            return Redirect::to("/login?error=server-error").into_response();
        }

        // The database lookup failed, so we cannot determine whether the
        // authenticated member still exists or which organization they belong to.
        Err(error) => {
            tracing::error!(?error, member_id = session.member_id,"failed to load member and organization for authenticated request");
            return Redirect::to("/login?error=server-error").into_response();
        }
    };

    // Construct the application-level authenticated request context.
    //
    // `Session` remains an internal authentication concern. Downstream
    // handlers receive `AuthenticatedUser` instead, which contains only the
    // identity and organization context they need.
    let authenticated_user = AuthenticatedUser {
        member_id: member_with_organization.member.id,
        organization: AssociatedOrganization {
            id: member_with_organization.organization.id,
            name: member_with_organization.organization.name,
        },
    };

    // Store the authenticated user context in the request's extensions so
    // downstream handlers can extract it without needing to read the session
    // cookie, access Redis, or query the organization themselves.
    request.extensions_mut().insert(authenticated_user);

    // Authentication and request-context resolution are complete.
    //
    // Pass the request to the next middleware or the final route handler.
    next.run(request).await
}