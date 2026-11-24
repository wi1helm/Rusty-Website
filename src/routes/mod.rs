// routes/mod.rs (top-level router)
//! Top-level HTTP router.
//!
//! Combines the HTML page routes (server-rendered pages the browser
//! navigates to directly) with the `/api/*` routes (form submission
//! targets that perform actions like signup/login), and serves static assets.

use crate::config::{CookieConfig, CookieSameSite};
use crate::{Context, api};
use axum::Router;
use axum::response::{IntoResponse, Redirect, Response};
use axum::routing::get;
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use tower_http::services::ServeDir;

mod errors;
mod images;
mod profile;
mod protected;
mod public;

/// Builds the complete application router.
///
/// `context` is attached once here via `.with_state(...)`, which is why
/// this function, unlike `api::router()`, returns a plain `Router` rather
/// than `Router<Context>`. Handlers extract only the dependencies they need
/// from this root state through `FromRef`.
pub fn router(context: Context) -> Router {
    Router::new()
        //TODO: Maintain a list of reserved slugs for top-level application routes.
        // Organization slugs must not collide with routes such as /login, /signup,
        // /dashboard, /settings, /api, /assets, etc. When new top-level routes are
        // introduced, add their names to the reserved list before exposing them.
        // TODO: Add a logout endpoint (e.g. POST /api/auth/logout) that deletes the
        // session from Redis and clears the session cookie. Currently the SessionManager
        // delete() method exists but no route calls it, so users cannot log out.
        // TODO: Add security headers (Content-Security-Policy, X-Content-Type-Options,
        // X-Frame-Options, Strict-Transport-Security). Consider using a middleware like
        // tower-http's `SetHeader` layer. While Caddy may set some headers, the app
        // should provide defense-in-depth.
        // See: https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers#security_headers
        .merge(public::router())
        .merge(protected::router(context.clone()))
        .nest("/api", api::router(context.clone()))
        .route("/images/{*key}", get(images::get_image))
        // Expose the local `static/` directory as `/assets/*` over HTTP.
        // The URL path is intentionally `/assets` rather than `/static` so the
        // filesystem layout stays an internal implementation detail.
        // For example: `/assets/shared/styles.css` -> `static/shared/styles.css`.
        .nest_service("/assets", ServeDir::new("static"))
        .with_state(context)
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
            .get(axum::http::header::SET_COOKIE)
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
            .get(axum::http::header::SET_COOKIE)
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
