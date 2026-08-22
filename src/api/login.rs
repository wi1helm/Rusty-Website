// routes/pages/login.rs
use serde::Deserialize;
use axum::{extract::State, Form};
use axum::response::Redirect;
use crate::Context;
use crate::api::validation::{is_valid_email, is_valid_password};
use crate::auth::{AuthenticationError, LoginCredentials};
use crate::auth::session::SessionError;

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
// The struct that handles server side validation of input
struct LoginValidator<'a> {
    input: &'a LoginRequest,
}
// The validator for login requests.
impl LoginValidator<'_> {
    fn validate(&self) -> Result<ValidatedLogin, LoginValidationError> {
        if !is_valid_email(&self.input.email) {
            return Err(LoginValidationError::InvalidEmail);
        }
        if !is_valid_password(&self.input.password) {
            return Err(LoginValidationError::InvalidPassword);
        }

        Ok(ValidatedLogin {
            email: self.input.email.clone(),
            password: self.input.password.clone(),
        })
    }
}

pub async fn login(State(context): State<Context>, Form(input): Form<LoginRequest>, ) -> impl axum::response::IntoResponse {
    let validated = match (LoginValidator { input: &input }).validate() {
        Ok(v) => v,
        Err(LoginValidationError::InvalidEmail) => return Redirect::to("/login?error=invalid-credentials"),
        Err(LoginValidationError::InvalidPassword) => return Redirect::to("/login?error=invalid-credentials")
    };

    let credentials = LoginCredentials {email: validated.email, password: validated.password};

    match context.authenticator.login(credentials).await {
        Ok(session) => {
            // TODO: Set the session cookie and redirect to /dashboard.
            // TODO(test-only): this puts session data directly in the URL so we
            // can visually confirm login -> SessionManager::create -> Redis
            // round-trips correctly, without needing to inspect Redis/Postgres
            // directly. This is NOT how the session should reach the client in
            // the real implementation -- it belongs in an HTTP-only cookie, set
            // via CookieJar, not a URL query string. Remove this once cookie
            // handling + a real /dashboard route exist.
            Redirect::to(&format!(
                "/dashboard?session_id={}&member_id={}&organization_id={}",
                session.id, session.member_id, session.organization_id
            ))
        }
        Err(AuthenticationError::InvalidCredentials) => Redirect::to("/login?error=invalid-credentials"),
        Err(AuthenticationError::Database(_)) => Redirect::to("/login?error=server-error"),

        // Credentials were correct, but we couldn't establish a session
        // (e.g. Redis unavailable). The account/credentials are fine —
        // this is an infrastructure failure, not the user's fault.
        Err(AuthenticationError::Session(session_err)) => {
            match &session_err {
                SessionError::Cache(_) => tracing::warn!("session cache error during login: {session_err}"),
                SessionError::Serialization(_) => tracing::error!("session serialization bug during login: {session_err}"),
            }
            Redirect::to("/login?error=server-error")
        }
    }
}
