// main.rs
//! Application entry point.
//!
//! Loads configuration from the environment, connects to Postgres,
//! builds the shared [`Context`], and starts the Axum HTTP server.

mod database;
mod routes;
mod api;

use std::sync::Arc;
use database::Database;

/// Shared application state, injected into every Axum handler via
/// [`axum::extract::State`].
///
/// `database` is type-erased behind `Arc<dyn Database>` so handlers
/// depend only on the [`Database`] trait, not a concrete implementation
#[derive(Clone)]
pub struct Context {
    pub database: Arc<dyn Database>
}
#[tokio::main]
async fn main() {
    // Get database url from environmental variables set in compose.yaml.
    // Panic if none is set!
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    // Create the database using the postgres implementation.
    // Wait for connection and panic if connection fails.
    let database = database::PostgresDatabase::new(&database_url)
        .await
        .expect("failed to connect to database");

    let context = Context {
        database: Arc::new(database)
    };

    let app = routes::router(context);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}