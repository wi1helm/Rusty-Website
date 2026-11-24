//! Application construction and runtime.
//!
//! App::build initializes the application's dependencies and HTTP router.
//! App::run starts the HTTP server.

use axum::Router;
use std::sync::Arc;
use std::{error::Error, net::SocketAddr};
use thiserror::Error;
use tracing::info;

use crate::{
    Context,
    auth::{Authenticator, session::SessionManager},
    config::AppConfig,
    routes,
    services::profile::ProfileService,
    storage::{
        cache::{Cache, redis::RedisCache},
        database::{MemberRepository, OrganizationRepository, PositionRepository, ProfileRepository, postgres::PostgresDatabase},
        object::{ObjectStorage, s3::S3ObjectStorage},
    },
};

type BoxError = Box<dyn Error + Send + Sync>;

pub struct App {
    listener: tokio::net::TcpListener,
    router: Router,
}

impl App {
    /// Builds the application and initializes all dependencies.
    pub async fn build(config: AppConfig) -> Result<Self, AppError> {
        info!(environment = ?config.environment, "Building application");

        let database = Arc::new(Self::build_database(&config).await?);
        let organization_repository: Arc<dyn OrganizationRepository> = database.clone();
        let member_repository: Arc<dyn MemberRepository> = database.clone();
        let profile_repository: Arc<dyn ProfileRepository> = database.clone();
        let position_repository: Arc<dyn PositionRepository> = database;

        let cache = Self::build_cache(&config).await?;
        let object_storage = Self::build_object_storage(&config).await?;

        let session_manager = Self::build_session_manager(cache.clone());
        let authenticator = Self::build_authenticator(member_repository.clone(), session_manager.clone());

        let profile_service = Arc::new(ProfileService::new(organization_repository.clone(), profile_repository.clone(), position_repository.clone(), object_storage.clone()));

        let context = Context {
            organization_repository,
            member_repository,
            profile_repository,
            position_repository,
            profile_service,
            public_object_storage: object_storage,
            authenticator,
            session_manager,
            cookies: config.cookies,
        };

        let router = routes::router(context);

        let address = SocketAddr::new(config.server.host, config.server.port);

        let listener = tokio::net::TcpListener::bind(address)
            .await
            .map_err(AppError::ServerBind)?;

        info!(address = %listener.local_addr().map_err(AppError::ServerBind)?, "Application built successfully");

        Ok(Self { listener, router })
    }

    /// Starts the HTTP server.
    pub async fn run(self) -> Result<(), AppError> {
        let address = self.listener.local_addr().map_err(AppError::ServerBind)?;

        info!(%address, "HTTP server started");

        axum::serve(self.listener, self.router)
            .await
            .map_err(AppError::Server)
    }

    // ─────────────────────────────────────────
    // Infrastructure
    // ─────────────────────────────────────────

    async fn build_database(config: &AppConfig) -> Result<PostgresDatabase, AppError> {
        info!("Connecting to PostgreSQL");

        let database = PostgresDatabase::new(&config.database.url)
            .await
            .map_err(|error| AppError::Database(Box::new(error)))?;

        info!("Connected to PostgreSQL");

        Ok(database)
    }

    async fn build_cache(config: &AppConfig) -> Result<Arc<dyn Cache>, AppError> {
        info!("Connecting to Redis");

        let cache = RedisCache::new(&config.cache.url)
            .await
            .map_err(|error| AppError::Cache(Box::new(error)))?;

        info!("Connected to Redis");

        Ok(Arc::new(cache))
    }

    async fn build_object_storage(config: &AppConfig) -> Result<Arc<dyn ObjectStorage>, AppError> {
        info!("Connecting to object storage");

        let storage = S3ObjectStorage::new(
            &config.object_storage.endpoint,
            config.object_storage.bucket.clone(),
            &config.object_storage.access_key,
            &config.object_storage.secret_key,
            config.object_storage.public_url.clone(),
        )
        .await
        .map_err(|error| AppError::ObjectStorage(Box::new(error)))?;

        info!("Object storage initialized");

        Ok(Arc::new(storage))
    }

    // ─────────────────────────────────────────
    // Application services
    // ─────────────────────────────────────────

    fn build_session_manager(cache: Arc<dyn Cache>) -> Arc<SessionManager> {
        info!("Initializing session manager");
        Arc::new(SessionManager::new(cache))
    }

    fn build_authenticator(members: Arc<dyn MemberRepository>, session_manager: Arc<SessionManager>) -> Arc<Authenticator> {
        info!("Initializing authenticator");
        Arc::new(Authenticator::new(members, session_manager))
    }
}

// ─────────────────────────────────────────────
// Application errors
// ─────────────────────────────────────────────

#[derive(Debug, Error)]
pub enum AppError {
    #[error("Failed to initialize PostgreSQL")]
    Database(#[source] BoxError),

    #[error("Failed to initialize Redis")]
    Cache(#[source] BoxError),

    #[error("Failed to initialize object storage")]
    ObjectStorage(#[source] BoxError),

    #[error("Failed to bind HTTP server")]
    ServerBind(#[source] std::io::Error),

    #[error("HTTP server failed")]
    Server(#[source] std::io::Error),
}
