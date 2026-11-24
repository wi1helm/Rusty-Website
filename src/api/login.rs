// routes/pages/login.rs
use crate::api::validation::{is_valid_email, is_valid_password};
use crate::auth::Authenticator;
use crate::auth::session::SessionError;
use crate::auth::{AuthenticationError, LoginCredentials};
use crate::config::CookieConfig;
use crate::routes::redirect_with_session_cookie;
use axum::response::{IntoResponse, Redirect};
use axum::{Form, extract::State};
use serde::Deserialize;
use std::sync::Arc;

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

        Ok(ValidatedLogin {
            email: self.email.clone(),
            password: self.password.clone(),
        })
    }
}

// TODO(AUDIT): [HIGH] Throttle repeated login attempts (AUDIT-001)
// Problem: The public login handler accepts repeated credential attempts without a rate limit.
// Impact: Attackers can brute-force credentials and drive expensive password verification work.
// Suggested fix: Add distributed per-IP/account throttling and alerting without revealing account state.
// Reference: POST /api/auth/login; Authenticator::login.
pub async fn login(State(authenticator): State<Arc<Authenticator>>, State(cookies): State<CookieConfig>, Form(input): Form<LoginRequest>) -> impl IntoResponse {
    let validated = match input.validate() {
        Ok(v) => v,
        Err(LoginValidationError::InvalidEmail) => { return Redirect::to("/login?error=invalid-credentials").into_response(); }
        Err(LoginValidationError::InvalidPassword) => { return Redirect::to("/login?error=invalid-credentials").into_response(); }
    };

    let credentials = LoginCredentials {
        email: validated.email,
        password: validated.password,
    };

    match authenticator.login(credentials).await {
        Ok(session) => redirect_with_session_cookie("/dashboard", &session.id, &cookies),
        Err(AuthenticationError::InvalidCredentials) => { Redirect::to("/login?error=invalid-credentials").into_response() }
        Err(AuthenticationError::Database(_)) => { Redirect::to("/login?error=server-error").into_response() }

        // Credentials were correct, but we couldn't establish a session
        // (e.g. Redis unavailable). The account/credentials are fine —
        // this is an infrastructure failure, not the user's fault.
        Err(AuthenticationError::Session(session_err)) => {
            match &session_err {
                SessionError::Cache(_) => { tracing::warn!("session cache error during login: {session_err}") }
                SessionError::Serialization(_) => { tracing::error!("session serialization bug during login: {session_err}") }
            }
            Redirect::to("/login?error=server-error").into_response()
        }
        Err(AuthenticationError::PasswordHash(error)) => {
            tracing::error!(?error, "password verification failed");
            Redirect::to("/login?error=server-error").into_response()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_login_with_valid_credentials_format() {
        let request = LoginRequest {
            email: "member@example.com".to_owned(),
            password: "long-enough-password".to_owned(),
        };

        let validated = request.validate().expect("valid credentials should pass validation");

        assert_eq!(validated.email, "member@example.com");
        assert_eq!(validated.password, "long-enough-password");
    }

    #[test]
    fn rejects_invalid_email() {
        let request = LoginRequest {
            email: "not-an-email".to_owned(),
            password: "long-enough-password".to_owned(),
        };

        assert!(matches!(request.validate(), Err(LoginValidationError::InvalidEmail)));
    }

    #[test]
    fn rejects_invalid_password() {
        let request = LoginRequest {
            email: "member@example.com".to_owned(),
            password: "short".to_owned(),
        };

        assert!(matches!(request.validate(), Err(LoginValidationError::InvalidPassword)));
    }
}
