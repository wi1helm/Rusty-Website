use crate::Context;
use axum::{Router, routing::post};

mod creation;
mod login;
mod protected;
mod validation;

/// Builds the `/api/*` routes.
///
/// Returns a `Router<Context>` rather than a fully built `Router` because
/// this is nested into the top-level router, which is the one that
/// actually attaches `Context` via `.with_state(...)`.
pub fn router(context: Context) -> Router<Context> {
    Router::new()
        // Public API routes.
        //
        // These do not require an authenticated session.
        // TODO(AUDIT): [LOW] Provide a secure password recovery flow (AUDIT-008)
        // Problem: No password-reset endpoint or public recovery page is registered.
        // Impact: A user who forgets the owner password cannot regain organization access.
        // Suggested fix: Add single-use, expiring reset tokens, verified delivery, and abuse limits.
        // Reference: Public authentication routes; templates/public/login.html.
        .route("/organizations/create", post(creation::signup))
        .route("/auth/login", post(login::login))
        // Protected API routes.
        //
        // The nested router applies authentication middleware to all
        // routes defined inside it.
        .nest("/profile", protected::router(context))
}
