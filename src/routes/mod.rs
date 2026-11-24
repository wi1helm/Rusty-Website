// routes/mod.rs (top-level router)
//! Top-level HTTP router.
//!
//! Combines the HTML page routes (server-rendered pages the browser
//! navigates to directly) with the `/api/*` routes (form submission
//! targets that perform actions like signup/login), and serves static assets.

use crate::config::{CookieConfig, CookieSameSite};
use crate::{Context, api};
use axum::Router;
use axum::extract::Request;
use axum::http::{HeaderValue, StatusCode, header};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Redirect, Response};
use axum::routing::get;
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use tower_http::services::ServeDir;
use url::Url;

mod errors;
mod images;
mod profile;
mod protected;
mod public;

const API_PATH: &str = "/api";
const IMAGES_PATH: &str = "/images/{*key}";
const ASSETS_PATH: &str = "/assets";

pub(crate) fn is_reserved_organization_slug(slug: &str) -> bool {
    [
        public::LOGIN_PATH,
        public::CREATE_PATH,
        public::ORGANIZATION_PATH,
        protected::DASHBOARD_PATH,
        protected::PROFILE_PATH,
        API_PATH,
        IMAGES_PATH,
        ASSETS_PATH,
    ]
    .iter()
    .any(|path| {
        path.strip_prefix('/').and_then(|path| path.split('/').next()) == Some(slug)
    })
}

/// Builds the complete application router.
///
/// `context` is attached once here via `.with_state(...)`, which is why
/// this function, unlike `api::router()`, returns a plain `Router` rather
/// than `Router<Context>`. Handlers extract only the dependencies they need
/// from this root state through `FromRef`.
pub fn router(context: Context) -> Router {
    Router::new()
        // TODO(AUDIT): [MEDIUM] Implement an authenticated logout route (AUDIT-003)
        // Problem: No route deletes the Redis session or clears the session cookie.
        // Impact: Users on shared devices cannot revoke their session and remain authenticated until expiry.
        // Suggested fix: Add a same-origin POST logout handler that deletes the session and expires the cookie.
        // Reference: src/auth/session.rs; protected navigation; session TTL is 24 hours.
        .merge(public::router())
        .merge(protected::router(context.clone()))
        .nest(API_PATH, api::router(context.clone()))
        .route(IMAGES_PATH, get(images::get_image))
        // Expose the local `static/` directory as `/assets/*` over HTTP.
        // The URL path is intentionally `/assets` rather than `/static` so the
        // filesystem layout stays an internal implementation detail.
        // For example: `/assets/shared/styles.css` -> `static/shared/styles.css`.
        .nest_service(ASSETS_PATH, ServeDir::new("static"))
        .with_state(context)
        .layer(middleware::from_fn(csrf_protection))
        .layer(middleware::map_response(add_security_headers))
}

/// Reject state-changing requests unless the browser identifies them as same-origin.
async fn csrf_protection(request: Request, next: Next) -> Response {
    let method = request.method();
    let state_changing = method != axum::http::Method::GET && method != axum::http::Method::HEAD && method != axum::http::Method::OPTIONS;

    if state_changing && !is_same_origin_request(&request) { return StatusCode::FORBIDDEN.into_response(); }

    next.run(request).await
}

fn is_same_origin_request(request: &Request) -> bool {
    if request.headers().get("sec-fetch-site").is_some_and(|value| value.to_str().ok() != Some("same-origin")) {
        return false;
    }

    let Some(source) = request
        .headers()
        .get(header::ORIGIN)
        .or_else(|| request.headers().get(header::REFERER))
        .and_then(|value| value.to_str().ok())
        .and_then(|value| Url::parse(value).ok())
    else {
        return false;
    };

    if !source.username().is_empty()
        || source.password().is_some()
        || !matches!(source.scheme(), "http" | "https")
    {
        return false;
    }

    request_origin(request).is_some_and(|expected| source.origin() == expected)
}

fn request_origin(request: &Request) -> Option<url::Origin> {
    let scheme = match request.headers().get("x-forwarded-proto") {
        Some(value) => value.to_str().ok()?.trim(),
        None => request.uri().scheme_str().unwrap_or("http"),
    };
    if !matches!(scheme, "http" | "https") { return None; }

    let authority = request
        .headers()
        .get(header::HOST)
        .and_then(|value| value.to_str().ok())
        .or_else(|| { request.uri().authority().map(|authority| authority.as_str()) })?
        .parse::<axum::http::uri::Authority>()
        .ok()?;

    let target = Url::parse(&format!("{scheme}://{authority}")).ok()?;
    Some(target.origin())
}

async fn add_security_headers(mut response: Response) -> Response {
    let headers = response.headers_mut();
    headers.insert(header::CONTENT_SECURITY_POLICY, HeaderValue::from_static("default-src 'self'; script-src 'self'; style-src 'self'; img-src 'self' https: data: blob:; base-uri 'self'; object-src 'none'; frame-ancestors 'none'; form-action 'self'"));
    headers.insert(header::X_CONTENT_TYPE_OPTIONS, HeaderValue::from_static("nosniff"));
    headers.insert(header::X_FRAME_OPTIONS, HeaderValue::from_static("DENY"));
    headers.insert(header::REFERRER_POLICY, HeaderValue::from_static("strict-origin-when-cross-origin"));
    headers.insert(header::STRICT_TRANSPORT_SECURITY, HeaderValue::from_static("max-age=31536000"));
    response
}

pub fn redirect_with_session_cookie(location: &str, session_id: &str, config: &CookieConfig) -> Response {
    let same_site = match config.same_site {
        CookieSameSite::Strict => SameSite::Strict,
        CookieSameSite::Lax => SameSite::Lax,
        CookieSameSite::None => SameSite::None,
    };

    let cookie = Cookie::build(("session", session_id.to_owned()))
        .http_only(config.http_only)
        .secure(config.secure)
        .same_site(same_site)
        .path("/")
        .build();

    let jar = CookieJar::new().add(cookie);

    (jar, Redirect::to(location)).into_response()
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::routing::post;
    use tower::ServiceExt;

    async fn post_status(origin: Option<&str>, referer: Option<&str>, fetch_site: Option<&str>) -> StatusCode {
        let app = Router::new()
            .route("/change", post(|| async { StatusCode::NO_CONTENT }))
            .layer(middleware::from_fn(csrf_protection));

        let mut request = axum::http::Request::builder()
            .method("POST")
            .uri("/change")
            .header(header::HOST, "app.example")
            .header("x-forwarded-proto", "https");
        if let Some(origin) = origin {
            request = request.header(header::ORIGIN, origin);
        }
        if let Some(referer) = referer {
            request = request.header(header::REFERER, referer);
        }
        if let Some(fetch_site) = fetch_site {
            request = request.header("sec-fetch-site", fetch_site);
        }

        app.oneshot(request.body(Body::empty()).unwrap())
            .await
            .unwrap()
            .status()
    }

    #[tokio::test]
    async fn csrf_protection_allows_same_origin_posts() {
        assert_eq!(
            post_status(Some("https://app.example"), None, Some("same-origin")).await,
            StatusCode::NO_CONTENT
        );
    }

    #[tokio::test]
    async fn csrf_protection_rejects_cross_origin_posts() {
        assert_eq!(
            post_status(Some("https://attacker.example"), None, Some("cross-site")).await,
            StatusCode::FORBIDDEN
        );
    }

    #[tokio::test]
    async fn csrf_protection_rejects_same_site_cross_origin_posts() {
        assert_eq!(
            post_status(Some("https://other.app.example"), None, Some("same-site")).await,
            StatusCode::FORBIDDEN
        );
    }

    #[tokio::test]
    async fn csrf_protection_uses_same_origin_referer_when_origin_is_absent() {
        assert_eq!(
            post_status(None, Some("https://app.example/profile"), None).await,
            StatusCode::NO_CONTENT
        );
    }

    #[tokio::test]
    async fn csrf_protection_rejects_posts_without_origin_or_referer() {
        assert_eq!(post_status(None, None, None).await, StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn security_headers_are_added_to_responses() {
        let response = add_security_headers(Response::new(Body::empty())).await;
        let headers = response.headers();

        assert_eq!(headers[header::CONTENT_SECURITY_POLICY], "default-src 'self'; script-src 'self'; style-src 'self'; img-src 'self' https: data: blob:; base-uri 'self'; object-src 'none'; frame-ancestors 'none'; form-action 'self'");
        assert_eq!(headers[header::X_CONTENT_TYPE_OPTIONS], "nosniff");
        assert_eq!(headers[header::X_FRAME_OPTIONS], "DENY");
        assert_eq!(headers[header::REFERRER_POLICY], "strict-origin-when-cross-origin");
        assert_eq!(headers[header::STRICT_TRANSPORT_SECURITY], "max-age=31536000");
    }

    #[tokio::test]
    async fn security_headers_are_added_to_csrf_rejections() {
        let app = Router::new()
            .route("/change", post(|| async { StatusCode::NO_CONTENT }))
            .layer(middleware::from_fn(csrf_protection))
            .layer(middleware::map_response(add_security_headers));
        let request = axum::http::Request::builder()
            .method("POST")
            .uri("/change")
            .header(header::HOST, "app.example")
            .header("x-forwarded-proto", "https")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();

        assert_eq!(response.status(), StatusCode::FORBIDDEN);
        assert_eq!(response.headers()[header::X_CONTENT_TYPE_OPTIONS], "nosniff");
        assert_eq!(response.headers()[header::X_FRAME_OPTIONS], "DENY");
        assert_eq!(response.headers()[header::STRICT_TRANSPORT_SECURITY], "max-age=31536000");
    }

    #[tokio::test]
    async fn csrf_protection_allows_safe_get_without_origin_headers() {
        let app = Router::new()
            .route("/read", get(|| async { StatusCode::NO_CONTENT }))
            .layer(middleware::from_fn(csrf_protection));
        let request = axum::http::Request::builder()
            .uri("/read")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();

        assert_eq!(response.status(), StatusCode::NO_CONTENT);
    }

    #[test]
    fn session_cookie_uses_configured_attributes() {
        let config = CookieConfig {
            secure: false,
            http_only: false,
            same_site: CookieSameSite::Strict,
        };
        let response = redirect_with_session_cookie("/dashboard", "test-session", &config);
        let cookie = response
            .headers()
            .get(header::SET_COOKIE)
            .unwrap()
            .to_str()
            .unwrap();

        assert!(
            !cookie
                .split(';')
                .any(|attribute| attribute.trim().eq_ignore_ascii_case("secure"))
        );
        assert!(
            !cookie
                .split(';')
                .any(|attribute| attribute.trim().eq_ignore_ascii_case("httponly"))
        );
        assert!(cookie.contains("SameSite=Strict"));
    }

    #[test]
    fn secure_session_cookie_sets_secure() {
        let config = CookieConfig {
            secure: true,
            http_only: true,
            same_site: CookieSameSite::Lax,
        };
        let response = redirect_with_session_cookie("/dashboard", "test-session", &config);
        let cookie = response
            .headers()
            .get(header::SET_COOKIE)
            .unwrap()
            .to_str()
            .unwrap();

        assert!(
            cookie
                .split(';')
                .any(|attribute| attribute.trim().eq_ignore_ascii_case("secure"))
        );
        assert!(
            cookie
                .split(';')
                .any(|attribute| attribute.trim().eq_ignore_ascii_case("httponly"))
        );
        assert!(cookie.contains("SameSite=Lax"));
    }
}
