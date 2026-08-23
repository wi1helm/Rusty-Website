// routes/mod.rs (top-level router)
//! Top-level HTTP router.
//!
//! Combines the HTML page routes (server-rendered pages the browser
//! navigates to directly) with the `/api/*` routes (form submission
//! targets that perform actions like signup/login), and serves static
//! frontend assets.

use axum::Router;
use axum::response::{IntoResponse, Redirect, Response};
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use tower_http::services::ServeDir;
use crate::{api, Context};

mod public;
mod protected;

/// Builds the complete application router.
///
/// `context` is attached once here via `.with_state(...)`, which is why
/// this function, unlike `api::router()` returns a plain `Router`
/// rather than `Router<Context>`. Everything nested under it (including
/// `api::router()`) inherits access to `Context` through Axum's state
/// extraction.
pub fn router(context: Context) -> Router {
    Router::new()
        .merge(public::router())
        .merge(protected::router(context.clone()))
        .nest("/api", api::router())
        .nest_service("/assets", ServeDir::new("frontend"))
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