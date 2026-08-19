use axum::{routing::get, Router};
use tower_http::services::ServeDir;
use crate::Context;

mod pages;

pub fn router(context: Context) -> Router {
    Router::new()
        .route("/", get(pages::main))
        .route("/organization", get(pages::organization))
        .route("/signup", get(pages::signup))
        .nest_service("/assets", ServeDir::new("frontend"))
        .with_state(context)
}
