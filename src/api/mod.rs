// api/mod.rs
use axum::{routing::post, Router};
use crate::Context;

mod signup;
mod validation;
mod login;

pub fn router() -> Router<Context> {
    Router::new()
        .route("/organizations/signup", post(signup::signup))
        .route("/auth/login", post(login::login))
}