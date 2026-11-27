use crate::Context;
use axum::{
    Router,
    extract::DefaultBodyLimit,
    extract::Request,
    middleware::{self, Next},
    response::Response,
    routing::post,
};

pub(crate) mod creation;
pub mod error;
pub(crate) mod login;
mod protected;
mod validation;

#[derive(Clone)]
pub(crate) struct ApiRequest;

pub(crate) async fn mark_as_api_request(mut request: Request, next: Next) -> Response {
    request.extensions_mut().insert(ApiRequest);
    next.run(request).await
}

/// Builds the `/api/*` routes.
///
/// Returns a `Router<Context>` rather than a fully built `Router` because
/// this is nested into the top-level router, which is the one that
/// actually attaches `Context` via `.with_state(...)`.
pub fn router(context: Context) -> Router<Context> {
    let login_rate_limiter = context.rate_limiter.clone();
    let signup_rate_limiter = context.rate_limiter.clone();

    Router::new()
        // Public API routes.
        //
        // These do not require an authenticated session.
        // TODO(AUDIT): [LOW] Provide a secure password recovery flow (AUDIT-008)
        // Problem: No password-reset endpoint or public recovery page is registered.
        // Impact: A user who forgets the owner password cannot regain organization access.
        // Suggested fix: Add single-use, expiring reset tokens, verified delivery, and abuse limits.
        // Reference: Public authentication routes; templates/public/login.html.
        //TODO Move out the magic numbers? or just snaller or more generic seems long and messy now.
        .route("/organizations/create", post(creation::create).layer(DefaultBodyLimit::max(16 * 1024)).route_layer(middleware::from_fn_with_state(signup_rate_limiter, crate::middleware::rate_limit::signup_ip_limit)))
        .route("/auth/login", post(login::login).layer(DefaultBodyLimit::max(16 * 1024)).route_layer(middleware::from_fn_with_state(login_rate_limiter, crate::middleware::rate_limit::login_ip_limit)))
        // Protected API routes.
        //
        // The nested router applies authentication middleware to all
        // routes defined inside it.
        .nest("/profile", protected::router(context))
        .layer(middleware::from_fn(mark_as_api_request))
}
