use axum::{Router, middleware, routing::get};

use crate::Context;
use crate::auth::middleware::{AuthState, require_auth};
use axum::extract::FromRef;

mod dashboard;
mod profile;

pub(super) const DASHBOARD_PATH: &str = "/dashboard";
pub(super) const PROFILE_PATH: &str = "/profile";

/// Routes that require an authenticated session.
///
/// The authentication middleware is attached to this router, meaning every
/// route defined here must have a valid session before its handler is called.
///
/// This lets us avoid adding authentication logic to every individual
/// protected handler.
pub fn router(context: Context) -> Router<Context> {
    let auth_state = AuthState::from_ref(&context);

    Router::new()
        .route(DASHBOARD_PATH, get(dashboard::get_page))
        .route(PROFILE_PATH, get(profile::get_page))
        .layer(middleware::from_fn_with_state(auth_state, require_auth))
}
