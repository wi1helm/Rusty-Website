use crate::api::error::api_error;
use crate::auth::session::SessionError;
use crate::auth::session::SessionManager;
use crate::auth::{AssociatedOrganization, AuthenticatedUser};
use crate::storage::database::MemberRepository;
use crate::{Context, api};
use axum::{
    extract::{FromRef, Request, State},
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Redirect, Response},
};
use axum_extra::extract::cookie::CookieJar;
use std::sync::Arc;

/// The database and session store required only while authenticating requests.
#[derive(Clone)]
pub struct AuthState {
    members: Arc<dyn MemberRepository>,
    session_manager: Arc<SessionManager>,
}

impl FromRef<Context> for AuthState {
    fn from_ref(context: &Context) -> Self {
        Self { members: context.member_repository.clone(), session_manager: context.session_manager.clone() }
    }
}

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
pub async fn require_auth(State(state): State<AuthState>, jar: CookieJar, mut request: Request, next: Next) -> Response {
    let is_api_request = request.extensions().get::<api::ApiRequest>().is_some();
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
        return create_authentication_response(is_api_request, StatusCode::UNAUTHORIZED, "authentication-required", "Please sign in to continue.");
    };

    let session_id = cookie.value();

    // Look up the session using the ID supplied by the browser.
    //
    // The session itself is stored server-side. A successfully retrieved
    // session establishes which member and organization this request belongs
    // to.
    let session = match state.session_manager.get(session_id).await {
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
            return create_authentication_response(is_api_request, StatusCode::UNAUTHORIZED, "authentication-required", "Please sign in to continue.");
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
            return create_authentication_response(is_api_request, StatusCode::SERVICE_UNAVAILABLE, "service-unavailable", "The service is temporarily unavailable. Please try again.");
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
            tracing::error!("session serialization error during authentication: {error}");
            return create_authentication_response(is_api_request, StatusCode::INTERNAL_SERVER_ERROR, "server-error", "Something went wrong. Please try again.");
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
    let member_with_organization = match state.members.find_member_with_organization_by_member_id(session.member_id).await {
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
            //TODO remove stale cookie and stale Redis value as this session id is invalid
            tracing::warn!(member_id = session.member_id, "authenticated session references a missing member");
            return create_authentication_response(is_api_request, StatusCode::INTERNAL_SERVER_ERROR, "server-error", "Something went wrong. Please try again.");
        }

        // The database lookup failed, so we cannot determine whether the
        // authenticated member still exists or which organization they belong to.
        Err(error) => {
            tracing::error!(?error, member_id = session.member_id, "failed to load member and organization for authenticated request");
            return create_authentication_response(is_api_request, StatusCode::INTERNAL_SERVER_ERROR, "server-error", "Something went wrong. Please try again.");
        }
    };

    // Construct the application-level authenticated request context.
    //
    // `Session` remains an internal authentication concern. Downstream
    // handlers receive `AuthenticatedUser` instead, which contains only the
    // identity and organization context they need.
    let authenticated_user = AuthenticatedUser { member_id: member_with_organization.member.id, organization: AssociatedOrganization { id: member_with_organization.organization.id, name: member_with_organization.organization.name } };

    // Store the authenticated user context in the request's extensions so
    // downstream handlers can extract it without needing to read the session
    // cookie, access Redis, or query the organization themselves.
    request.extensions_mut().insert(authenticated_user);

    // Authentication and request-context resolution are complete.
    //
    // Pass the request to the next middleware or the final route handler.
    next.run(request).await
}

fn create_authentication_response(is_api_request: bool, status: StatusCode, code: &'static str, message: &'static str) -> Response {
    if is_api_request {
        return api_error(status, code, message);
    }
    if (status == StatusCode::UNAUTHORIZED) {
        return Redirect::to("/login").into_response();
    }
    status.into_response()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::{
        cache::{Cache, CacheError},
        database::{DatabaseError, DatabaseMemberWithOrganization, MemberCredentials, MemberRepository},
    };
    use async_trait::async_trait;
    use axum::{
        Router,
        body::Body,
        http::{Request, StatusCode, header},
        middleware,
        routing::get,
    };
    use std::time::Duration;
    use tower::ServiceExt;

    struct EmptyCache;

    #[async_trait]
    impl Cache for EmptyCache {
        async fn get(&self, _key: &str) -> Result<Option<Vec<u8>>, CacheError> {
            Ok(None)
        }

        async fn set(&self, _key: &str, _value: &[u8], _ttl: Duration) -> Result<(), CacheError> {
            Ok(())
        }

        async fn delete(&self, _key: &str) -> Result<(), CacheError> {
            Ok(())
        }

        async fn increment_with_expiry(&self, _key: &str, ttl: Duration) -> Result<(u64, Duration), CacheError> {
            Ok((1, ttl))
        }

        async fn decrement(&self, _key: &str) -> Result<(), CacheError> {
            Ok(())
        }
    }

    struct EmptyMembers;

    #[async_trait]
    impl MemberRepository for EmptyMembers {
        async fn find_member_credentials_by_email(&self, _email: &str) -> Result<Option<MemberCredentials>, DatabaseError> {
            Ok(None)
        }

        async fn find_member_with_organization_by_member_id(&self, _member_id: i64) -> Result<Option<DatabaseMemberWithOrganization>, DatabaseError> {
            Ok(None)
        }
    }

    fn protected_router() -> Router {
        let state = AuthState { members: Arc::new(EmptyMembers), session_manager: Arc::new(SessionManager::new(Arc::new(EmptyCache))) };

        let api_router = Router::new().route("/api/private", get(|| async { StatusCode::NO_CONTENT })).layer(middleware::from_fn_with_state(state.clone(), require_auth)).layer(middleware::from_fn(crate::api::mark_as_api_request));

        Router::new().route("/private", get(|| async { StatusCode::NO_CONTENT })).layer(middleware::from_fn_with_state(state, require_auth)).merge(api_router)
    }

    async fn request_private(cookie: Option<&str>) -> Response {
        request_private_path("/private", cookie).await
    }

    async fn request_private_path(path: &str, cookie: Option<&str>) -> Response {
        let mut request = Request::builder().uri(path);
        if let Some(cookie) = cookie {
            request = request.header(header::COOKIE, cookie);
        }

        protected_router().oneshot(request.body(Body::empty()).unwrap()).await.unwrap()
    }

    #[tokio::test]
    async fn protected_route_redirects_when_session_cookie_is_missing() {
        let response = request_private(None).await;

        assert_eq!(response.status(), StatusCode::SEE_OTHER);
        assert_eq!(response.headers()[header::LOCATION], "/login");
    }

    #[tokio::test]
    async fn protected_route_redirects_when_session_does_not_exist() {
        let response = request_private(Some("session=unknown-session")).await;

        assert_eq!(response.status(), StatusCode::SEE_OTHER);
        assert_eq!(response.headers()[header::LOCATION], "/login");
    }

    #[tokio::test]
    async fn protected_api_returns_a_structured_unauthorized_error() {
        let response = request_private_path("/api/private", None).await;

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        assert_eq!(response.headers()[header::CONTENT_TYPE], "application/json");
        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let body: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(body["error"]["code"], "authentication-required");
    }
}
