use axum::{
    routing::get,
    Router,
};

use crate::Context;
mod pages;
mod signup;
mod login;

pub fn router() -> Router<Context> {
    Router::new()
        .route("/", get(pages::main))
        .route("/login", get(login::login))
        .route("/signup", get(signup::signup))
        .route("/organizations", get(pages::organization))
}
