use axum::{middleware, routing::post, Router};
use axum::extract::DefaultBodyLimit;
use crate::auth::middleware::require_auth;
use crate::Context;

mod profile;
mod image;


/// Builds API routes that require an authenticated session.
///
/// Authentication middleware is applied to the entire router, so every
/// endpoint defined here is guaranteed to pass through `require_auth`
/// before its handler is called.
pub fn router(context: Context) -> Router<Context> {
    Router::new()
        .route("/edit", post(profile::edit))
        .route("/publish", post(profile::publish))
        .route("/image", post(image::image::upload).layer(DefaultBodyLimit::max(image::MAX_REQUEST_SIZE)))
        .layer(middleware::from_fn_with_state(context, require_auth))
}