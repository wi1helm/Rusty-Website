use std::{net::SocketAddr, sync::Arc};

use axum::{
    extract::{ConnectInfo, Request, State},
    middleware::Next,
    response::Response,
};

use crate::{
    api::error::{api_error, rate_limit_error},
    services::rate_limiter::RateLimiter,
};

pub(crate) async fn login_ip_limit(State(limiter): State<Arc<RateLimiter>>, request: Request, next: Next) -> Response {
    enforce_ip_limit(limiter, request, next, false).await
}

pub(crate) async fn signup_ip_limit(State(limiter): State<Arc<RateLimiter>>, request: Request, next: Next) -> Response {
    enforce_ip_limit(limiter, request, next, true).await
}

async fn enforce_ip_limit(limiter: Arc<RateLimiter>, request: Request, next: Next, signup: bool) -> Response {
    let Some(ConnectInfo(peer)) = request.extensions().get::<ConnectInfo<SocketAddr>>() else {
        tracing::error!("client socket address unavailable for public rate limiting");
        return api_error(axum::http::StatusCode::SERVICE_UNAVAILABLE, "service-unavailable", "The service is temporarily unavailable. Please try again.");
    };

    let ip = limiter.client_ip(*peer, request.headers().get("x-forwarded-for"));
    let result = if signup { limiter.check_signup_ip_request(ip).await } else { limiter.check_login_ip_request(ip).await };
    if let Err(error) = result {
        return rate_limit_error(error);
    }

    next.run(request).await
}
