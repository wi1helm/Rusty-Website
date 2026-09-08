use axum::{routing::post, Router};
use crate::Context;

mod creation;
mod validation;
mod login;
mod protected;

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
        .route("/organizations/create", post(creation::signup))
        .route("/auth/login", post(login::login))

        // Protected API routes.
        //
        // The nested router applies authentication middleware to all
        // routes defined inside it.
        .nest("/profile", protected::router(context))
}