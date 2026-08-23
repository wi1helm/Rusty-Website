use axum::{
    extract::{Request, State},
    middleware::Next,
    response::{IntoResponse, Redirect, Response},
};
use axum_extra::extract::cookie::CookieJar;
use crate::Context;
use crate::auth::session::SessionError;


/// Authentication middleware for protected routes.
///
/// The browser only sends the session ID in the `session` cookie. The actual
/// session data remains server-side in the session store (Redis as of writing).
///
/// The middleware is responsible for:
/// - reading the session cookie
/// - looking up the session in the session store
/// - rejecting requests that are not authenticated
/// - putting the resolved `Session` into request extensions
/// - passing authenticated requests to the actual route handler
///
/// This keeps authentication logic out of individual protected handlers.
/// A handler such as `dashboard` can simply extract the authenticated session
/// instead of knowing anything about cookies or Redis.
/// Note: this should only be put on protected routes
///
pub async fn require_auth(State(context): State<Context>, jar: CookieJar, mut request: Request, next: Next) -> Response {
    //TODO(security): Consider clearing the session cookie when the session is
    // missing or invalid. This avoids repeatedly sending a stale session ID.
    // This will require returning a response with an updated CookieJar rather
    // than only returning a Redirect.
    //TODO(security): Consider whether an expired/invalid session should be
    // distinguishable from a missing session cookie for logging/metrics.
    // The user-facing response should remain the same so we don't reveal
    // unnecessary authentication details.
    //TODO: Consider adding explicit logging/metrics for authentication failures
    // once application observability is in place. In particular, distinguish
    // missing sessions from cache failures and serialization failures.

    // The session cookie is the browser's proof that it has previously
    // established an authenticated session.
    //
    // No cookie means there is no session ID that we can use to authenticate
    // the request, so send the user to the login page.
    let Some(cookie) = jar.get("session") else {
        return Redirect::to("/login").into_response();
    };

    let session_id = cookie.value();

    // Look up the session using the ID supplied by the browser.
    //
    // The session itself is stored server-side. The cookie only contains the
    // random session ID and does not contain member or organization data.
    let session = match context.session_manager.get(session_id).await {
        // The session exists and was successfully loaded.
        //
        // This is the successful authentication path. We continue below and
        // attach the session to the request before passing it to the handler.
        Ok(Some(session)) => session,

        // Redis/cache lookup succeeded, but there is no session with this ID.
        //
        // This can happen when:
        // - the session expired
        // - the user logged out
        // - the session was deleted
        // - the cookie contains an invalid/old session ID
        //
        // This is not a server error. From the application's perspective,
        // the request simply isn't authenticated anymore.
        Ok(None) => {
            return Redirect::to("/login").into_response();
        }

        // The session store itself could not be accessed.
        //
        // This is different from Ok(None): we don't know whether the user
        // actually has a valid session because the server failed to check.
        //
        // Cache errors are generally expected to be transient infrastructure
        // problems, such as Redis being unavailable or a connection timing out.
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
            //The session ID is effectively a bearer credential. Someone who obtains a valid session ID may be able to authenticate as that user.
            // IMPORTANT TO NOT LOG the id, application logs could potentially be access
            tracing::error!("session serialization error during authentication: {error}");

            return Redirect::to("/login?error=server-error").into_response();
        }
    };

    // Authentication succeeded.
    //
    // Store the resolved Session in the request's extensions so downstream
    // handlers can extract it without having to read the cookie or query
    // Redis themselves.
    request.extensions_mut().insert(session);

    // Authentication middleware is finished. Pass the request to the next
    // middleware/handler in the route chain.
    next.run(request).await
}
