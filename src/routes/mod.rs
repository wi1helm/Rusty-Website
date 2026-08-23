// routes/mod.rs (top-level router)
//! Top-level HTTP router.
//!
//! Combines the HTML page routes (server-rendered pages the browser
//! navigates to directly) with the `/api/*` routes (form submission
//! targets that perform actions like signup/login), and serves static
//! frontend assets.

use axum::{routing::get, Router};
use axum::{response::{IntoResponse, Redirect, Response}, };
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use tower_http::services::ServeDir;
use crate::{Context, api};

mod pages;

/// Builds the complete application router.
///
/// `context` is attached once here via `.with_state(...)`, which is why
/// this function, unlike `api::router()` returns a plain `Router`
/// rather than `Router<Context>`. Everything nested under it (including
/// `api::router()`) inherits access to `Context` through Axum's state
/// extraction.
pub fn router(context: Context) -> Router {
    Router::new()
        // Server-rendered HTML pages. These are what a browser GETs
        // when a user navigates directly (typing a URL, following a
        // link, form redirect targets, etc).
        .route("/", get(pages::main))
        .route("/organization", get(pages::organization))
        .route("/signup", get(pages::signup))
        .route("/login", get(pages::login))

        // Everything the browser's <form> elements POST to lives under
        // /api, kept separate from the page routes above.
        .nest("/api", api::router())

        // Static assets (CSS, JS, images) served directly from the
        // `frontend` directory on disk, mounted at /assets.
        .nest_service("/assets", ServeDir::new("frontend"))

        // Makes `Context` (database, cache, authenticator) available to
        // every handler above via `State(context): State<Context>`.
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