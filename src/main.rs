// main.rs
//! Application entry point.
//!
//! Loads configuration from the environment, connects to Postgres,
//! builds the shared [`Context`], and starts the Axum HTTP server.

mod routes;
mod api;
mod storage;
mod auth;
mod config;

use std::sync::Arc;
use storage::database::Database;
use storage::database::postgres::PostgresDatabase;
use storage::cache::Cache;
use storage::cache::redis::RedisCache;
use storage::object::ObjectStorage;
use storage::object::s3::S3ObjectStorage;
use crate::auth::{Authenticator, session::SessionManager};

/// Shared application state, injected into every Axum handler via
/// [`axum::extract::State`].
///
/// `database` is type-erased behind `Arc<dyn Database>` so handlers
/// depend only on the [`Database`] trait, not a concrete implementation
/// `cache` is type-erased behind `Arc<dyn Cache>` so handlers
///  depend only on the [`Cache`] trait, not a concrete implementation
#[derive(Clone)]
pub struct Context {
    pub database: Arc<dyn Database>,
    pub cache: Arc<dyn Cache>,
    pub public_object_storage: Arc<dyn ObjectStorage>,
    pub authenticator: Arc<Authenticator>,
    pub session_manager: Arc<SessionManager>,
}

#[tokio::main]
async fn main() {
    // Get database and cache urls from environmental variables set in compose.yaml.
    // Panic if any is not present!
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let cache_url = std::env::var("CACHE_URL").expect("CACHE_URL must be set");
    // Get S3 object storage configuration from environmental variables set in compose.yaml.
    // Panic if any is not present!
    let s3_endpoint = std::env::var("S3_ENDPOINT").expect("S3_ENDPOINT must be set");
    let s3_bucket = std::env::var("S3_BUCKET").expect("S3_BUCKET must be set");
    let s3_access_key = std::env::var("S3_ACCESS_KEY").expect("S3_ACCESS_KEY must be set");
    let s3_secret_key = std::env::var("S3_SECRET_KEY").expect("S3_SECRET_KEY must be set");
    //let s3_region = std::env::var("S3_REGION").expect("S3_REGION must be set");
    let s3_public_url = std::env::var("S3_PUBLIC_URL").expect("S3_PUBLIC_URL must be set");
    // Create the database using the Postgres implementation.
    // Arc allows the database handle to be cheaply shared between multiple
    // parts of the application, such as Context and Authenticator.
    // dyn Database hides the concrete Postgres implementation behind the
    // Database trait, so the rest of the application does not depend on Postgres.
    let database: Arc<dyn Database> = Arc::new(
        PostgresDatabase::new(&database_url)
            .await
            .expect("failed to connect to database")
    );

    // Create the cache using the Redis implementation.
    // Arc allows the cache handle to be shared between multiple parts of
    // the application without copying the underlying connection/state.
    // dyn Cache hides the concrete Redis implementation behind the Cache trait.
    let cache: Arc<dyn Cache> = Arc::new(
        RedisCache::new(&cache_url)
            .await
            .expect("failed to connect to cache")
    );
    // Create the object storage using the S3 implementation
    // This is for image storing and based on url, endpoint etc could point to different places.
    let public_object_storage: Arc<dyn ObjectStorage> = Arc::new(
        S3ObjectStorage::new(s3_endpoint.as_str(), s3_bucket, s3_access_key.as_str(), s3_secret_key.as_str(), s3_public_url)
            .await
            .expect("failed to create storage object")
    );


    // The SessionManager needs access to the cache (redis) at the sametime as
    // the rest of the application, so clone the Arc handle rather than the cache itself.
    let session_manager = Arc::new(SessionManager::new(cache.clone()));

    // The Authenticator needs access to the same database as the rest of
    // the application, so clone the Arc handle rather than the database itself.
    // it also needs session_manager for session management
    let authenticator = Arc::new(
        Authenticator::new(database.clone(), session_manager.clone())
    );
    // Create the application context.
    let context = Context { database, cache, public_object_storage, authenticator, session_manager};

    // Create the router for this application
    let app = routes::router(context);
    // bind the listener to a port and ip and start the server the application.
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}