// api/mod.rs
use axum::{routing::post, Router};
use crate::Context;

mod signup;
mod validation;

pub fn router() -> Router<Context> {
    Router::new()
        .route("/signup", post(signup::signup))
}