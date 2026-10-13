// routes/mod.rs (top-level router)
//! Top-level HTTP router.
//!
//! Combines the HTML page routes (server-rendered pages the browser
//! navigates to directly) with the `/api/*` routes (form submission
//! targets that perform actions like signup/login), and serves static assets.

use axum::Router;
use axum::response::{IntoResponse, Redirect, Response};
use axum::routing::get;
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use tower_http::services::ServeDir;
use crate::{api, Context};

mod public;
mod protected;
mod errors;
mod images;
mod profile;

/// Builds the complete application router.
///
/// `context` is attached once here via `.with_state(...)`, which is why
/// this function, unlike `api::router()` returns a plain `Router`
/// rather than `Router<Context>`. Everything nested under it (including
/// `api::router()`) inherits access to `Context` through Axum's state
/// extraction.
pub fn router(context: Context) -> Router {
    Router::new()
        //TODO: Maintain a list of reserved slugs for top-level application routes.
        // Organization slugs must not collide with routes such as /login, /signup,
        // /dashboard, /settings, /api, /assets, etc. When new top-level routes are
        // introduced, add their names to the reserved list before exposing them.
        .merge(public::router())
        .merge(protected::router(context.clone()))
        .nest("/api", api::router(context.clone()))
        .route("/images/{*key}", get(images::get_image), )
        // Expose the local `static/` directory as `/assets/*` over HTTP.
        // The URL path is intentionally `/assets` rather than `/static` so the
        // filesystem layout stays an internal implementation detail.
        // For example: `/assets/shared/styles.css` -> `static/shared/styles.css`.
        .nest_service("/assets", ServeDir::new("static"))
        .with_state(context)
}




pub fn redirect_with_session_cookie(location: &str, session_id: &str, ) -> Response {
    let cookie = Cookie::build(("session", session_id.to_owned()))
        .http_only(true)
        .secure(true)
        .same_site(SameSite::Lax)
        .path("/")
        .build();

    let jar = CookieJar::new().add(cookie);

    (jar, Redirect::to(location)).into_response()
}