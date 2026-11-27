//! Application construction and runtime.
//!
//! App::build initializes the application's dependencies and HTTP router.
//! App::run starts the HTTP server.

use axum::Router;
use std::sync::Arc;
use std::{error::Error, net::SocketAddr};
use thiserror::Error;
use tracing::info;

use crate::config::{RateLimitConfig, TrustedProxyNetwork};
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

        let router = Self::build_router(AppDependencies { organization_repository, member_repository, profile_repository, position_repository, cache, object_storage, cookies: config.cookies, rate_limits: config.rate_limits, rate_limit_hmac_key: config.rate_limit_hmac_key, trusted_proxies: config.trusted_proxies, argon2_max_concurrency: config.argon2_max_concurrency });

        let address = SocketAddr::new(config.server.host, config.server.port);

        let listener = tokio::net::TcpListener::bind(address).await.map_err(AppError::ServerBind)?;

        info!(address = %listener.local_addr().map_err(AppError::ServerBind)?, "Application built successfully");

        Ok(Self { listener, router })
    }

    /// Starts the HTTP server.
    pub async fn run(self) -> Result<(), AppError> {
        let address = self.listener.local_addr().map_err(AppError::ServerBind)?;

        info!(%address, "HTTP server started");

        axum::serve(self.listener, self.router.into_make_service_with_connect_info::<SocketAddr>()).await.map_err(AppError::Server)
    }

    // ─────────────────────────────────────────
    // Infrastructure
    // ─────────────────────────────────────────

    async fn build_database(config: &AppConfig) -> Result<PostgresDatabase, AppError> {
        info!("Connecting to PostgreSQL");

        let database = PostgresDatabase::new(&config.database.url).await.map_err(|error| AppError::Database(Box::new(error)))?;

        info!("Connected to PostgreSQL");

        Ok(database)
    }

    async fn build_cache(config: &AppConfig) -> Result<Arc<dyn Cache>, AppError> {
        info!("Connecting to Redis");

        let cache = RedisCache::new(&config.cache.url).await.map_err(|error| AppError::Cache(Box::new(error)))?;

        info!("Connected to Redis");

        Ok(Arc::new(cache))
    }

    async fn build_object_storage(config: &AppConfig) -> Result<Arc<dyn ObjectStorage>, AppError> {
        info!("Connecting to object storage");

        let storage = S3ObjectStorage::new(&config.object_storage.endpoint, config.object_storage.bucket.clone(), &config.object_storage.access_key, &config.object_storage.secret_key).await.map_err(|error| AppError::ObjectStorage(Box::new(error)))?;

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

    fn build_authenticator(members: Arc<dyn MemberRepository>, session_manager: Arc<SessionManager>, passwords: Arc<crate::auth::password::PasswordService>) -> Arc<Authenticator> {
        info!("Initializing authenticator");
        Arc::new(Authenticator::new(members, session_manager, passwords))
    }

    fn build_router(dependencies: AppDependencies) -> Router {
        let passwords = Arc::new(crate::auth::password::PasswordService::new(dependencies.argon2_max_concurrency));
        let rate_limiter = Arc::new(crate::services::rate_limiter::RateLimiter::new(dependencies.cache.clone(), dependencies.rate_limits, dependencies.rate_limit_hmac_key, dependencies.trusted_proxies));
        let session_manager = Self::build_session_manager(dependencies.cache);
        let authenticator = Self::build_authenticator(dependencies.member_repository.clone(), session_manager.clone(), passwords.clone());

        let profile_service = Arc::new(ProfileService::new(dependencies.organization_repository.clone(), dependencies.profile_repository.clone(), dependencies.position_repository.clone(), dependencies.object_storage.clone()));

        let context = Context {
            organization_repository: dependencies.organization_repository,
            member_repository: dependencies.member_repository,
            profile_repository: dependencies.profile_repository,
            position_repository: dependencies.position_repository,
            profile_service,
            public_object_storage: dependencies.object_storage,
            authenticator,
            password_service: passwords,
            rate_limiter,
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
    rate_limits: RateLimitConfig,
    rate_limit_hmac_key: String,
    trusted_proxies: Vec<TrustedProxyNetwork>,
    argon2_max_concurrency: usize,
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
            cache::CacheError,
            database::{CreateOrganizationError, DatabaseError, DatabaseImage, DatabaseLink, DatabaseMemberWithOrganization, DatabaseOrganization, DatabasePosition, DatabasePositionConnection, DatabaseProfile, MemberCredentials, NewMember, NewOrganization, NewPosition, NewPositionConnection, NewProfileImage, NewProfileLink, UpdateProfile},
            object::ObjectStorageError,
        },
    };
    use async_trait::async_trait;
    use axum::{
        body::Body,
        extract::ConnectInfo,
        http::{Request, StatusCode, header},
    };
    use std::{
        collections::HashMap,
        net::SocketAddr,
        sync::{
            Mutex,
            atomic::{AtomicUsize, Ordering},
        },
        time::{Duration, Instant},
    };
use tower::ServiceExt;

    #[derive(Default)]
    struct FakeDependencies {
        cache: Mutex<HashMap<String, (Vec<u8>, Instant)>>,
        login_credentials: Mutex<Option<MemberCredentials>>,
        login_calls: AtomicUsize,
        signup_writes: AtomicUsize,
        cache_unavailable: std::sync::atomic::AtomicBool,
        login_database_unavailable: std::sync::atomic::AtomicBool,
    }

    #[async_trait]
    impl OrganizationRepository for FakeDependencies {
        async fn create_organization_with_owner_and_profile(&self, _organization: NewOrganization, _owner: NewMember) -> Result<(i64, i64), CreateOrganizationError> {
            self.signup_writes.fetch_add(1, Ordering::SeqCst);
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
            self.login_calls.fetch_add(1, Ordering::SeqCst);
            if self.login_database_unavailable.load(Ordering::SeqCst) {
                return Err(DatabaseError::Other("test database failure".to_owned()));
            }
            Ok(self.login_credentials.lock().unwrap().clone())
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

        async fn find_position_connections_by_organization_id(&self, _organization_id: i64) -> Result<Vec<DatabasePositionConnection>, DatabaseError> {
            Ok(Vec::new())
        }

        async fn replace_graph_by_organization_id(&self, _organization_id: i64, _positions: Vec<NewPosition>, _connections: Vec<NewPositionConnection>) -> Result<(), DatabaseError> {
            Ok(())
        }
    }

    #[async_trait]
    impl Cache for FakeDependencies {
        async fn get(&self, key: &str) -> Result<Option<Vec<u8>>, CacheError> {
            self.check_cache()?;
            let mut cache = self.cache.lock().unwrap();
            match cache.get(key) {
                Some((_, expires_at)) if *expires_at <= Instant::now() => {
                    cache.remove(key);
                    Ok(None)
                }
                Some((value, _)) => Ok(Some(value.clone())),
                None => Ok(None),
            }
        }

        async fn set(&self, key: &str, value: &[u8], ttl: Duration) -> Result<(), CacheError> {
            self.check_cache()?;
            self.cache.lock().unwrap().insert(key.to_owned(), (value.to_vec(), Instant::now() + ttl));
            Ok(())
        }

        async fn delete(&self, key: &str) -> Result<(), CacheError> {
            self.check_cache()?;
            self.cache.lock().unwrap().remove(key);
            Ok(())
        }

        async fn increment_with_expiry(&self, key: &str, ttl: Duration) -> Result<(u64, Duration), CacheError> {
            self.check_cache()?;
            let mut cache = self.cache.lock().unwrap();
            let now = Instant::now();
            let (mut count, expires_at) = cache.get(key).filter(|(_, expires_at)| *expires_at > now).map(|(value, expires_at)| (std::str::from_utf8(value).ok().and_then(|value| value.parse::<u64>().ok()).unwrap_or_default(), *expires_at)).unwrap_or((0, now + ttl));
            count += 1;
            cache.insert(key.to_owned(), (count.to_string().into_bytes(), expires_at));
            Ok((count, expires_at.saturating_duration_since(now)))
        }

        async fn decrement(&self, key: &str) -> Result<(), CacheError> {
            self.check_cache()?;
            let mut cache = self.cache.lock().unwrap();
            let Some((value, expires_at)) = cache.get(key).cloned() else {
                return Ok(());
            };
            let count = std::str::from_utf8(&value).ok().and_then(|value| value.parse::<u64>().ok()).unwrap_or_default().saturating_sub(1);
            if count == 0 {
                cache.remove(key);
            } else {
                cache.insert(key.to_owned(), (count.to_string().into_bytes(), expires_at));
            }
            Ok(())
        }
    }

    impl FakeDependencies {
        fn check_cache(&self) -> Result<(), CacheError> {
            if self.cache_unavailable.load(Ordering::SeqCst) { Err(CacheError::Unavailable) } else { Ok(()) }
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

    fn test_router_with_dependencies() -> (Router, Arc<FakeDependencies>) {
        let fake = Arc::new(FakeDependencies::default());
        let dependencies = AppDependencies {
            organization_repository: fake.clone(),
            member_repository: fake.clone(),
            profile_repository: fake.clone(),
            position_repository: fake.clone(),
            cache: fake.clone(),
            object_storage: fake.clone(),
            cookies: CookieConfig { secure: false, http_only: true, same_site: CookieSameSite::Lax },
            rate_limits: crate::config::RateLimitConfig::default(),
            rate_limit_hmac_key: "unit tests use a non-production hmac key".to_owned(),
            trusted_proxies: Vec::new(),
            argon2_max_concurrency: 1,
        };

        (App::build_router(dependencies), fake)
    }

    fn test_router() -> Router {
        test_router_with_dependencies().0
    }

    fn with_peer(request: Request<Body>, ip: &str) -> Request<Body> {
        let mut request = request;
        let peer = SocketAddr::new(ip.parse().unwrap(), 43210);
        request.extensions_mut().insert(ConnectInfo(peer));
        request
    }

    fn login_request(email: &str, password: &str, ip: &str) -> Request<Body> {
        with_peer(
            Request::builder().method("POST").uri("/api/auth/login").header(header::HOST, "app.example").header(header::ORIGIN, "http://app.example").header(header::CONTENT_TYPE, "application/json").body(Body::from(serde_json::json!({"email": email, "password": password}).to_string())).unwrap(),
            ip,
        )
    }

    fn signup_request(email: &str, ip: &str) -> Request<Body> {
        with_peer(
            Request::builder()
                .method("POST")
                .uri("/api/organizations/create")
                .header(header::HOST, "app.example")
                .header(header::ORIGIN, "http://app.example")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    serde_json::json!({
                        "name": "Example Organization",
                        "email": email,
                        "password": "long-enough-password",
                        "confirm": "long-enough-password"
                    })
                    .to_string(),
                ))
                .unwrap(),
            ip,
        )
    }

    fn malformed_json_request(path: &str, ip: &str) -> Request<Body> {
        with_peer(
            Request::builder().method("POST").uri(path).header(header::HOST, "app.example").header(header::ORIGIN, "http://app.example").header(header::CONTENT_TYPE, "application/json").body(Body::from("{")).unwrap(),
            ip,
        )
    }

    #[tokio::test]
    async fn composed_router_serves_public_routes_and_protects_dashboard() {
        let app = test_router();

        let login = app.clone().oneshot(Request::builder().uri("/login").body(Body::empty()).unwrap()).await.unwrap();

        assert_eq!(login.status(), StatusCode::OK);
        assert!(login.headers()[header::CONTENT_TYPE].to_str().unwrap().starts_with("text/html"));
        assert!(login.headers().contains_key(header::CONTENT_SECURITY_POLICY));
        let body = axum::body::to_bytes(login.into_body(), usize::MAX).await.unwrap();
        let html = std::str::from_utf8(&body).unwrap();
        assert!(html.contains("action=\"/api/auth/login\""));

        let dashboard = app.clone().oneshot(Request::builder().uri("/dashboard").body(Body::empty()).unwrap()).await.unwrap();

        assert_eq!(dashboard.status(), StatusCode::SEE_OTHER);
        assert_eq!(dashboard.headers()[header::LOCATION], "/login");
        assert!(dashboard.headers().contains_key(header::STRICT_TRANSPORT_SECURITY));
    }

    #[tokio::test]
    async fn login_api_returns_a_structured_invalid_credentials_error() {
        let request = login_request("member@example.com", "long-enough-password", "192.0.2.1");

        let response = test_router().oneshot(request).await.unwrap();

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        assert_eq!(response.headers()[header::CONTENT_TYPE], "application/json");
        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let body: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(body["error"]["code"], "invalid-credentials");
        assert_eq!(body["error"]["message"], "Incorrect email or password.");
    }

    #[tokio::test]
    async fn repeated_failed_logins_are_throttled_across_client_ips_before_authentication() {
        let (app, fake) = test_router_with_dependencies();

        for attempt in 1..=5 {
            let response = app.clone().oneshot(login_request("shared@example.com", "wrong-password", &format!("198.51.100.{attempt}"))).await.unwrap();
            assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        }

        let response = app.clone().oneshot(login_request("shared@example.com", "wrong-password", "198.51.100.6")).await.unwrap();

        assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
        assert!(response.headers().contains_key(header::RETRY_AFTER));
        assert_eq!(fake.login_calls.load(Ordering::SeqCst), 5, "throttled request must not reach credential lookup");
    }

    #[tokio::test]
    async fn successful_login_does_not_consume_a_failed_account_attempt() {
        let (app, fake) = test_router_with_dependencies();
        let password = "a-correct-password";
        let password_service = crate::auth::password::PasswordService::new(1);
        let password_hash = password_service.hash_password(password.to_owned()).await.unwrap();
        *fake.login_credentials.lock().unwrap() = Some(MemberCredentials { member_id: 7, password_hash });

        let success = app.clone().oneshot(login_request("known@example.com", password, "192.0.2.10")).await.unwrap();
        assert_eq!(success.status(), StatusCode::OK);

        for attempt in 1..=5 {
            let response = app.clone().oneshot(login_request("known@example.com", "wrong-password", &format!("192.0.2.{}", 10 + attempt))).await.unwrap();
            assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        }

        let response = app.clone().oneshot(login_request("known@example.com", "wrong-password", "192.0.2.20")).await.unwrap();
        assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
    }

    #[tokio::test]
    async fn database_failures_do_not_count_as_failed_password_attempts() {
        let (app, fake) = test_router_with_dependencies();
        fake.login_database_unavailable.store(true, Ordering::SeqCst);

        let response = app.clone().oneshot(login_request("member@example.com", "wrong-password", "192.0.2.40")).await.unwrap();
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
        fake.login_database_unavailable.store(false, Ordering::SeqCst);

        for attempt in 1..=5 {
            let response = app.clone().oneshot(login_request("member@example.com", "wrong-password", &format!("192.0.2.{}", 40 + attempt))).await.unwrap();
            assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        }

        let response = app.clone().oneshot(login_request("member@example.com", "wrong-password", "192.0.2.46")).await.unwrap();
        assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
    }

    #[tokio::test]
    async fn signup_api_sets_session_cookie_and_returns_redirect_destination() {
        let request = signup_request("owner@example.com", "192.0.2.2");

        let response = test_router().oneshot(request).await.unwrap();

        assert_eq!(response.status(), StatusCode::CREATED);
        assert!(response.headers()[header::SET_COOKIE].to_str().unwrap().contains("HttpOnly"));
        assert!(response.headers()[header::SET_COOKIE].to_str().unwrap().contains("SameSite=Lax"));
        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let body: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(body["redirectTo"], "/dashboard");
    }

    #[tokio::test]
    async fn repeated_signup_attempts_are_limited_by_email_before_registration() {
        let (app, fake) = test_router_with_dependencies();

        for ip in ["192.0.2.10", "192.0.2.11", "192.0.2.12"] {
            let response = app.clone().oneshot(signup_request("owner@example.com", ip)).await.unwrap();
            assert_eq!(response.status(), StatusCode::CREATED);
        }

        let response = app.clone().oneshot(signup_request("owner@example.com", "192.0.2.13")).await.unwrap();

        assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
        assert!(response.headers().contains_key(header::RETRY_AFTER));
        assert_eq!(fake.signup_writes.load(Ordering::SeqCst), 3, "throttled request must not reach organization creation");
    }

    #[tokio::test]
    async fn rate_limiter_cache_failures_fail_closed_for_login_and_signup() {
        let (app, fake) = test_router_with_dependencies();
        fake.cache_unavailable.store(true, Ordering::SeqCst);

        let login = app.clone().oneshot(login_request("member@example.com", "password-long-enough", "192.0.2.30")).await.unwrap();
        let signup = app.clone().oneshot(signup_request("owner@example.com", "192.0.2.31")).await.unwrap();

        assert_eq!(login.status(), StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(signup.status(), StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(fake.login_calls.load(Ordering::SeqCst), 0);
        assert_eq!(fake.signup_writes.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn malformed_api_json_is_normalized_to_a_structured_error() {
        let request = with_peer(
            Request::builder().method("POST").uri("/api/auth/login").header(header::HOST, "app.example").header(header::ORIGIN, "http://app.example").header(header::CONTENT_TYPE, "application/json").body(Body::from("{")).unwrap(),
            "192.0.2.3",
        );

        let response = test_router().oneshot(request).await.unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        assert_eq!(response.headers()[header::CONTENT_TYPE], "application/json");
        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let body: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(body["error"]["code"], "invalid-request");
    }

    #[tokio::test]
    async fn malformed_public_requests_are_still_counted_by_the_ip_middleware() {
        let login_app = test_router();
        for _ in 0..10 {
            let response = login_app.clone().oneshot(malformed_json_request("/api/auth/login", "203.0.113.20")).await.unwrap();
            assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        }
        let response = login_app.oneshot(malformed_json_request("/api/auth/login", "203.0.113.20")).await.unwrap();
        assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);

        let signup_app = test_router();
        for _ in 0..5 {
            let response = signup_app.clone().oneshot(malformed_json_request("/api/organizations/create", "203.0.113.21")).await.unwrap();
            assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        }
        let response = signup_app.oneshot(malformed_json_request("/api/organizations/create", "203.0.113.21")).await.unwrap();
        assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
    }

    #[tokio::test]
    async fn public_rate_limits_fail_closed_when_socket_peer_is_missing() {
        let request = Request::builder().method("POST").uri("/api/auth/login").header(header::HOST, "app.example").header(header::ORIGIN, "http://app.example").header(header::CONTENT_TYPE, "application/json").body(Body::from(r#"{"email":"member@example.com","password":"long-enough-password"}"#)).unwrap();

        let response = test_router().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    }

    #[tokio::test]
    async fn browser_navigation_errors_render_html_instead_of_framework_text() {
        let response = test_router().oneshot(Request::builder().uri("/not/a-profile").header(header::ACCEPT, "text/html").body(Body::empty()).unwrap()).await.unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND, "location: {:?}", response.headers().get(header::LOCATION));
        assert!(response.headers()[header::CONTENT_TYPE].to_str().unwrap().starts_with("text/html"));
        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let html = std::str::from_utf8(&body).unwrap();
        assert!(html.contains("Page not found"));
        assert!(html.contains("class=\"not-found\""));
        assert!(html.contains("href=\"/organization\""));
        assert!(!html.contains("404 Not Found"));
    }

    #[tokio::test]
    async fn api_csrf_errors_remain_structured_json() {
        let response = test_router().oneshot(Request::builder().method("POST").uri("/api/auth/login").header(header::ACCEPT, "application/json").header(header::CONTENT_TYPE, "application/json").body(Body::empty()).unwrap()).await.unwrap();

        assert_eq!(response.status(), StatusCode::FORBIDDEN);
        assert_eq!(response.headers()[header::CONTENT_TYPE], "application/json");
        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let body: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(body["error"]["code"], "forbidden");
    }

    #[tokio::test]
    async fn creation_page_has_an_api_form_action_without_error_query_state() {
        let response = test_router().oneshot(Request::builder().uri("/create").body(Body::empty()).unwrap()).await.unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let html = std::str::from_utf8(&body).unwrap();
        assert!(!html.contains("An organization with that URL already exists."));
        assert!(html.contains("action=\"/api/organizations/create\""));
        assert!(!html.contains("data-api-action"));
    }

    #[tokio::test]
    async fn composed_router_serves_favicon_without_not_found() {
        let response = test_router().oneshot(Request::builder().uri("/favicon.ico").body(Body::empty()).unwrap()).await.unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.headers()[header::CONTENT_TYPE], "image/svg+xml");
        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        assert!(std::str::from_utf8(&body).unwrap().contains("<svg"));
    }

    #[tokio::test]
    async fn composed_router_wires_api_state_and_csrf_middleware() {
        let request = with_peer(
            Request::builder().method("POST").uri("/api/auth/login").header(header::HOST, "app.example").header(header::ORIGIN, "http://app.example").header(header::CONTENT_TYPE, "application/json").body(Body::from(r#"{"email":"member@example.com","password":"long-enough-password"}"#)).unwrap(),
            "192.0.2.4",
        );

        let response = test_router().oneshot(request).await.unwrap();

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        assert_eq!(response.headers()[header::CONTENT_TYPE], "application/json");
        assert!(response.headers().contains_key(header::X_CONTENT_TYPE_OPTIONS));
    }

    #[tokio::test]
    async fn composed_router_returns_json_for_unauthenticated_api_requests() {
        let request = Request::builder().method("POST").uri("/api/profile/update").header(header::HOST, "app.example").header(header::ORIGIN, "http://app.example").header(header::ACCEPT, "text/html").header(header::CONTENT_TYPE, "application/json").body(Body::from(r#"{"display_name":"Example","about":"","links":[]}"#)).unwrap();

        let response = test_router().oneshot(request).await.unwrap();

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        assert_eq!(response.headers()[header::CONTENT_TYPE], "application/json");
        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let body: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(body["error"]["code"], "authentication-required");
    }

    #[tokio::test]
    async fn unknown_api_routes_return_json_not_the_html_not_found_page() {
        let response = test_router().oneshot(Request::builder().uri("/api/unknown").header(header::ACCEPT, "text/html").body(Body::empty()).unwrap()).await.unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        assert_eq!(response.headers()[header::CONTENT_TYPE], "application/json");
        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let body: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(body["error"]["code"], "not-found");
    }
}
