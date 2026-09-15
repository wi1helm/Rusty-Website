use axum::{middleware, routing::post, Router};
use axum::extract::DefaultBodyLimit;
use axum::http::StatusCode;
use crate::auth::middleware::require_auth;
use crate::Context;
use crate::storage::database::DatabaseProfile;

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

pub async fn get_profile(context: &Context, organization_id: i64, ) -> Result<DatabaseProfile, StatusCode> {
    match context.database.find_profile_by_organization_id(organization_id).await {
        Ok(Some(profile)) => Ok(profile),

        Ok(None) => {
            tracing::error!(organization_id, "organization exists without a profile");
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }

        Err(error) => {
            tracing::error!(?error,organization_id,"failed to load organization profile");
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}