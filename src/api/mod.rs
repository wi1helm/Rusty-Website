// api/mod.rs
//! JSON/form API routes (as opposed to the HTML page routes in `routes/pages`).
//!
//! These are the endpoints that actually perform actions,
//! (signup,login, join etc( and are what the HTML forms in `pages` submit to.
//! They live under `/api` once nested into the main router (see the top-level router below).

use axum::{routing::post, Router};
use crate::Context;

mod signup;
mod validation;
mod login;

/// Builds the `/api/*` routes.
///
/// Returns a `Router<Context>` rather than a fully built `Router` because
/// this is nested into the top-level router, which is the one that
/// actually attaches `Context` via `.with_state(...)`.
pub fn router() -> Router<Context> {
    Router::new()
        // Creates a new organization + owner account, then logs the
        // owner in (see routes/api/signup.rs for the full flow).
        .route("/organizations/signup", post(signup::signup))

        // Verifies credentials and establishes a session for an
        // existing member (see routes/api/login.rs).
        .route("/auth/login", post(login::login))
}