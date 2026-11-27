// routes/pages/login.rs
use crate::Context;
use crate::api::error::{ApiSuccessResponse, api_error, rate_limit_error};
use crate::api::validation::{is_valid_email, is_valid_password};
use crate::auth::session::SessionError;
use crate::auth::{AuthenticationError, LoginCredentials};
use crate::routes::with_session_cookie;
use axum::extract::ConnectInfo;
use axum::http::HeaderMap;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::{Json, extract::State};
use serde::Deserialize;
use std::net::SocketAddr;

#[derive(Debug)]
pub enum LoginValidationError {
    InvalidEmail,
    InvalidPassword,
}

// The input data from input form. Raw data cant be trusted at all
#[derive(Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}
// The validated login still cannot be trusted 100% but is at least valid input to our standards
pub struct ValidatedLogin {
    pub email: String,
    pub password: String,
}

// The validator for login requests.
impl LoginRequest {
    fn validate(&self) -> Result<ValidatedLogin, LoginValidationError> {
        if !is_valid_email(&self.email) {
            return Err(LoginValidationError::InvalidEmail);
        }
        if !is_valid_password(&self.password) {
            return Err(LoginValidationError::InvalidPassword);
        }

        Ok(ValidatedLogin { email: self.email.clone(), password: self.password.clone() })
    }
}

// TODO(AUDIT): [HIGH] Throttle repeated login attempts (AUDIT-001)
// Problem: The public login handler accepts repeated credential attempts without a rate limit.
// Impact: Attackers can brute-force credentials and drive expensive password verification work.
// Suggested fix: Add distributed per-IP/account throttling and alerting without revealing account state.
// Reference: POST /api/auth/login; Authenticator::login.
pub async fn login(State(context): State<Context>, ConnectInfo(peer): ConnectInfo<SocketAddr>, headers: HeaderMap, Json(input): Json<LoginRequest>) -> impl IntoResponse {
    let validated = match input.validate() {
        Ok(v) => v,
        Err(LoginValidationError::InvalidEmail) => {
            return api_error(StatusCode::BAD_REQUEST, "invalid-credentials", "Enter a valid email address and password.");
        }
        Err(LoginValidationError::InvalidPassword) => {
            return api_error(StatusCode::BAD_REQUEST, "invalid-credentials", "Enter a valid email address and password.");
        }
    };

    let client_ip = context.rate_limiter.client_ip(peer, headers.get("x-forwarded-for"));
    if let Err(error) = context.rate_limiter.check_login_ip_account_request(client_ip, &validated.email).await {
        return rate_limit_error(error);
    }

    let reservation = match context.rate_limiter.reserve_login_account(&validated.email).await {
        Ok(reservation) => reservation,
        Err(error) => return rate_limit_error(error),
    };
    let credentials = LoginCredentials { email: validated.email, password: validated.password };

    match context.authenticator.login(credentials).await {
        Ok(session) => {
            if let Err(error) = reservation.release().await {
                return rate_limit_error(error);
            }
            with_session_cookie((StatusCode::OK, Json(ApiSuccessResponse::redirect_to("/dashboard"))).into_response(), &session.id, &context.cookies)
        }
        Err(AuthenticationError::InvalidCredentials) => {
            let backoff = reservation.record_failure();
            tokio::time::sleep(backoff).await;
            api_error(StatusCode::UNAUTHORIZED, "invalid-credentials", "Incorrect email or password.")
        }
        Err(AuthenticationError::Database(error)) => {
            if let Err(error) = reservation.release().await {
                return rate_limit_error(error);
            }
            tracing::error!(?error, "database error during login");
            api_error(StatusCode::INTERNAL_SERVER_ERROR, "server-error", "Something went wrong. Please try again.")
        }

        // Credentials were correct, but we couldn't establish a session
        // (e.g. Redis unavailable). The account/credentials are fine —
        // this is an infrastructure failure, not the user's fault.
        Err(AuthenticationError::Session(session_err)) => {
            if let Err(error) = reservation.release().await {
                return rate_limit_error(error);
            }
            match &session_err {
                SessionError::Cache(_) => {
                    tracing::warn!("session cache error during login: {session_err}")
                }
                SessionError::Serialization(_) => {
                    tracing::error!("session serialization bug during login: {session_err}")
                }
            }
            api_error(StatusCode::SERVICE_UNAVAILABLE, "service-unavailable", "The service is temporarily unavailable. Please try again.")
        }
        Err(AuthenticationError::PasswordHash(error)) => {
            if let Err(error) = reservation.release().await {
                return rate_limit_error(error);
            }
            tracing::error!(?error, "password verification failed");
            api_error(StatusCode::INTERNAL_SERVER_ERROR, "server-error", "Something went wrong. Please try again.")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_login_with_valid_credentials_format() {
        let request = LoginRequest { email: "member@example.com".to_owned(), password: "long-enough-password".to_owned() };

        let validated = request.validate().expect("valid credentials should pass validation");

        assert_eq!(validated.email, "member@example.com");
        assert_eq!(validated.password, "long-enough-password");
    }

    #[test]
    fn rejects_invalid_email() {
        let request = LoginRequest { email: "not-an-email".to_owned(), password: "long-enough-password".to_owned() };

        assert!(matches!(request.validate(), Err(LoginValidationError::InvalidEmail)));
    }

    #[test]
    fn rejects_invalid_password() {
        let request = LoginRequest { email: "member@example.com".to_owned(), password: "short".to_owned() };

        assert!(matches!(request.validate(), Err(LoginValidationError::InvalidPassword)));
    }
}
