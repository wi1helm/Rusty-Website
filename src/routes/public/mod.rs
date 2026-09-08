use axum::{routing::get, Router};
use crate::Context;
mod creation;
mod login;
mod organization;
mod profile;
/// Routes for pages that are publicly accessible without authentication.
///
/// Organization profiles are exposed at the root using their unique slug,
/// for example `/obama-robotics`.
pub fn router() -> Router<Context> {
    Router::new()
        .route("/login", get(login::get_page))
        .route("/create", get(creation::get_page))
        .route("/organization", get(organization::get_page))
        .route("/{slug}", get(profile::get_page))
}
