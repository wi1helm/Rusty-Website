use axum::{middleware, routing::get, Router, };

use crate::Context;
use crate::auth::middleware::require_auth;

mod dashboard;
mod profile;

/// Routes that require an authenticated session.
///
/// The authentication middleware is attached to this router, meaning every
/// route defined here must have a valid session before its handler is called.
///
/// This lets us avoid adding authentication logic to every individual
/// protected handler.
pub fn router(context: Context) -> Router<Context> {
    Router::new()
        .route("/dashboard", get(dashboard::get_page))
        .route("/profile", get(profile::get_page))
        .layer(middleware::from_fn_with_state(context, require_auth))
}
