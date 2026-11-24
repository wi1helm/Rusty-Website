mod api;
mod app;
mod auth;
mod config;
mod routes;
mod storage;

use axum::extract::FromRef;
use std::sync::Arc;

#[tokio::main]
async fn main() {
    if let Err(error) = run().await {
        eprintln!("Application error: {error:#}");
        std::process::exit(1);
    }
}

async fn run() -> Result<(), Box<dyn std::error::Error>> {
    // Load and validate application configuration.
    let config = config::AppConfig::from_env()?;

    // Initialize logging.
    tracing_subscriber::fmt().with_env_filter(&config.log_filter).init();
    tracing::info!(environment = ?config.environment, "Starting application");

    // Build application.
    let app = app::App::build(config).await?;

    // Start HTTP server.
    app.run().await?;

    Ok(())
}


/// Shared application state attached to the Axum router.
///
/// Handlers extract only the dependencies they need through `FromRef`.
#[derive(Clone)]
pub struct Context {
    pub database: Arc<dyn storage::database::Database>,
    pub public_object_storage: Arc<dyn storage::object::ObjectStorage>,
    pub authenticator: Arc<auth::Authenticator>,
    pub session_manager: Arc<auth::session::SessionManager>,
    pub cookies: config::CookieConfig,
}

impl FromRef<Context> for Arc<dyn storage::database::Database> {
    fn from_ref(context: &Context) -> Self { context.database.clone() }
}
impl FromRef<Context> for Arc<dyn storage::object::ObjectStorage> {
    fn from_ref(context: &Context) -> Self { context.public_object_storage.clone() }
}
impl FromRef<Context> for Arc<auth::Authenticator> {
    fn from_ref(context: &Context) -> Self { context.authenticator.clone() }
}
impl FromRef<Context> for config::CookieConfig {
    fn from_ref(context: &Context) -> Self { context.cookies.clone() }
}