use axum::{routing::get, Router};
use crate::Context;
mod signup;
mod login;
mod organization;

pub fn router() -> Router<Context> {
    Router::new()
        .route("/login", get(login::get_page))
        .route("/signup", get(signup::get_page))
        .route("/organization", get(organization::get_page))
}
