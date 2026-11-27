use crate::Context;
use axum::{Router, routing::get};
mod creation;
mod login;
mod organization;
mod profile;

pub(super) const LOGIN_PATH: &str = "/login";
pub(super) const CREATE_PATH: &str = "/create";
pub(super) const ORGANIZATION_PATH: &str = "/organization";

/// Routes for pages that are publicly accessible without authentication.
///
/// Organization profiles are exposed at the root using their unique slug,
/// for example `/obama-robotics`.
pub fn router() -> Router<Context> {
    Router::new().route(LOGIN_PATH, get(login::get_page)).route(CREATE_PATH, get(creation::get_page)).route(ORGANIZATION_PATH, get(organization::get_page)).route("/{slug}", get(profile::get_page))
}
