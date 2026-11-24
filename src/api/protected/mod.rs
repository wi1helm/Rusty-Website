use crate::Context;
use crate::auth::middleware::{AuthState, require_auth};
use crate::storage::database::{DatabaseProfile, ProfileRepository};
use axum::extract::DefaultBodyLimit;
use axum::extract::FromRef;
use axum::http::StatusCode;
use axum::{Router, middleware, routing::post};

mod image;
mod profile;

/// Builds API routes that require an authenticated session.
///
/// Authentication middleware is applied to the entire router, so every
/// endpoint defined here is guaranteed to pass through `require_auth`
/// before its handler is called.
pub fn router(context: Context) -> Router<Context> {
    let auth_state = AuthState::from_ref(&context);

    Router::new()
        .route("/update", post(profile::update))
        .route("/image", post(image::image::upload).layer(DefaultBodyLimit::max(image::MAX_REQUEST_SIZE)), )
        .layer(middleware::from_fn_with_state(auth_state, require_auth))
}

pub async fn get_profile(profiles: &dyn ProfileRepository, organization_id: i64) -> Result<DatabaseProfile, StatusCode> {
    match profiles.find_profile_by_organization_id(organization_id).await {
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
