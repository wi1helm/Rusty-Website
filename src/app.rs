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
    config::{AppConfig, CookieConfig},
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

        let router = Self::build_router(AppDependencies {
            organization_repository,
            member_repository,
            profile_repository,
            position_repository,
            cache,
            object_storage,
            cookies: config.cookies,
        });

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

    fn build_router(dependencies: AppDependencies) -> Router {
        let session_manager = Self::build_session_manager(dependencies.cache);
        let authenticator = Self::build_authenticator(dependencies.member_repository.clone(), session_manager.clone());

        let profile_service = Arc::new(ProfileService::new(
            dependencies.organization_repository.clone(),
            dependencies.profile_repository.clone(),
            dependencies.position_repository.clone(),
            dependencies.object_storage.clone(),
        ));

        let context = Context {
            organization_repository: dependencies.organization_repository,
            member_repository: dependencies.member_repository,
            profile_repository: dependencies.profile_repository,
            position_repository: dependencies.position_repository,
            profile_service,
            public_object_storage: dependencies.object_storage,
            authenticator,
            session_manager,
            cookies: dependencies.cookies,
        };

        routes::router(context)
    }
}

struct AppDependencies {
    organization_repository: Arc<dyn OrganizationRepository>,
    member_repository: Arc<dyn MemberRepository>,
    profile_repository: Arc<dyn ProfileRepository>,
    position_repository: Arc<dyn PositionRepository>,
    cache: Arc<dyn Cache>,
    object_storage: Arc<dyn ObjectStorage>,
    cookies: CookieConfig,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        config::CookieSameSite,
        storage::{
            cache::{CacheError},
            database::{
                CreateOrganizationError, DatabaseError, DatabaseImage, DatabaseLink,
                DatabaseMemberWithOrganization, DatabaseOrganization, DatabasePosition,
                DatabaseProfile, MemberCredentials, NewMember, NewOrganization,
                NewProfileImage, NewProfileLink, UpdateProfile,
            },
            object::ObjectStorageError,
        },
    };
    use async_trait::async_trait;
    use axum::{body::Body, http::{Request, StatusCode, header}};
    use std::time::Duration;
    use tower::ServiceExt;

    struct FakeDependencies;

    #[async_trait]
    impl OrganizationRepository for FakeDependencies {
        async fn create_organization_with_owner_and_profile(&self, _organization: NewOrganization, _owner: NewMember) -> Result<(i64, i64), CreateOrganizationError> {
            Ok((1, 1))
        }

        async fn find_organization_by_organization_id(&self, _organization_id: i64) -> Result<Option<DatabaseOrganization>, DatabaseError> {
            Ok(None)
        }

        async fn find_organization_by_slug(&self, _slug: &str) -> Result<Option<DatabaseOrganization>, DatabaseError> {
            Ok(None)
        }
    }

    #[async_trait]
    impl MemberRepository for FakeDependencies {
        async fn find_member_credentials_by_email(&self, _email: &str) -> Result<Option<MemberCredentials>, DatabaseError> {
            Ok(None)
        }

        async fn find_member_with_organization_by_member_id(&self, _member_id: i64) -> Result<Option<DatabaseMemberWithOrganization>, DatabaseError> {
            Ok(None)
        }
    }

    #[async_trait]
    impl ProfileRepository for FakeDependencies {
        async fn find_profile_by_organization_id(&self, _organization_id: i64) -> Result<Option<DatabaseProfile>, DatabaseError> {
            Ok(None)
        }

        async fn update_profile(&self, _organization_id: i64, _member_id: i64, _profile: UpdateProfile) -> Result<Option<DatabaseProfile>, DatabaseError> {
            Ok(None)
        }

        async fn update_profile_image(&self, _image: NewProfileImage) -> Result<Option<DatabaseImage>, DatabaseError> {
            Ok(None)
        }

        async fn update_profile_links(&self, _organization_id: i64, _links: Vec<NewProfileLink>) -> Result<Vec<DatabaseLink>, DatabaseError> {
            Ok(Vec::new())
        }
    }

    #[async_trait]
    impl PositionRepository for FakeDependencies {
        async fn find_positions_by_organization_id(&self, _organization_id: i64) -> Result<Vec<DatabasePosition>, DatabaseError> {
            Ok(Vec::new())
        }
    }

    #[async_trait]
    impl Cache for FakeDependencies {
        async fn get(&self, _key: &str) -> Result<Option<Vec<u8>>, CacheError> {
            Ok(None)
        }

        async fn set(&self, _key: &str, _value: &[u8], _ttl: Duration) -> Result<(), CacheError> {
            Ok(())
        }

        async fn delete(&self, _key: &str) -> Result<(), CacheError> {
            Ok(())
        }
    }

    #[async_trait]
    impl ObjectStorage for FakeDependencies {
        async fn put(&self, _key: &str, _data: Vec<u8>, _content_type: &str) -> Result<(), ObjectStorageError> {
            Ok(())
        }

        async fn get(&self, _key: &str) -> Result<Vec<u8>, ObjectStorageError> {
            Err(ObjectStorageError::NotFound)
        }

        async fn delete(&self, _key: &str) -> Result<(), ObjectStorageError> {
            Ok(())
        }

        fn url(&self, key: &str) -> String {
            format!("https://images.example/{key}")
        }
    }

    fn test_router() -> Router {
        let fake = Arc::new(FakeDependencies);
        let dependencies = AppDependencies {
            organization_repository: fake.clone(),
            member_repository: fake.clone(),
            profile_repository: fake.clone(),
            position_repository: fake.clone(),
            cache: fake.clone(),
            object_storage: fake,
            cookies: CookieConfig {
                secure: false,
                http_only: true,
                same_site: CookieSameSite::Lax,
            },
        };

        App::build_router(dependencies)
    }

    #[tokio::test]
    async fn composed_router_serves_public_routes_and_protects_dashboard() {
        let app = test_router();

        let login = app.clone().oneshot(Request::builder().uri("/login").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(login.status(), StatusCode::OK);
        assert!(login.headers()[header::CONTENT_TYPE].to_str().unwrap().starts_with("text/html"));
        assert!(login.headers().contains_key(header::CONTENT_SECURITY_POLICY));

        let dashboard = app.clone()
            .oneshot(Request::builder().uri("/dashboard").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(dashboard.status(), StatusCode::SEE_OTHER);
        assert_eq!(dashboard.headers()[header::LOCATION], "/login");
        assert!(dashboard.headers().contains_key(header::STRICT_TRANSPORT_SECURITY));
    }

    #[tokio::test]
    async fn composed_router_wires_api_state_and_csrf_middleware() {
        let request = Request::builder()
            .method("POST")
            .uri("/api/auth/login")
            .header(header::HOST, "app.example")
            .header(header::ORIGIN, "http://app.example")
            .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
            .body(Body::from("email=member%40example.com&password=long-enough-password"))
            .unwrap();

        let response = test_router().oneshot(request).await.unwrap();

        assert_eq!(response.status(), StatusCode::SEE_OTHER);
        assert_eq!(response.headers()[header::LOCATION], "/login?error=invalid-credentials");
        assert!(response.headers().contains_key(header::X_CONTENT_TYPE_OPTIONS));
    }
}
