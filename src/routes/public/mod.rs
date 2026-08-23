use axum::{
    routing::get,
    Router,
};

use crate::Context;
mod pages;

pub fn router() -> Router<Context> {
    Router::new()
        .route("/", get(pages::main))
        .route("/login", get(pages::login))
        .route("/signup", get(pages::signup))
        .route("/organizations", get(pages::organization))
}
