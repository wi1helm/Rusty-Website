// routes/pages/login.rs
use serde::Deserialize;
use axum::{extract::State, Form};
use axum::response::Redirect;
use crate::Context;
use crate::api::validation::{ValidationError, is_valid_email, is_valid_password};
use crate::database::DatabaseError;

#[derive(Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

pub struct ValidatedLogin {
    pub email: String,
    pub password: String,
}

struct LoginValidator<'a> {
    input: &'a LoginRequest,
}

impl LoginValidator<'_> {
    fn validate(&self) -> Result<ValidatedLogin, ValidationError> {
        if !is_valid_email(&self.input.email) {
            return Err(ValidationError::InvalidEmail);
        }
        if !is_valid_password(&self.input.password) {
            return Err(ValidationError::InvalidPassword);
        }

        Ok(ValidatedLogin {
            email: self.input.email.clone(),
            password: self.input.password.clone(),
        })
    }
}

pub async fn login(State(context): State<Context>, Form(input): Form<LoginRequest>) -> impl axum::response::IntoResponse {
    let validated = match (LoginValidator { input: &input }).validate() {
        Ok(v) => v,
        Err(ValidationError::InvalidEmail) => return Redirect::to("/login?error=invalid-credentials"),
        Err(ValidationError::InvalidPassword) => return Redirect::to("/login?error=invalid-credentials"),
        Err(ValidationError::InvalidName) => unreachable!("login has no name field"),
        Err(ValidationError::PasswordMismatch) => unreachable!("login has no confirm field"),
    };

    let member = match context.database.find_member_by_email(&validated.email).await {
        Ok(Some(member)) => member,
        Ok(None) => return Redirect::to("/login?error=invalid-credentials"),
        Err(DatabaseError::Sqlx(_)) => return Redirect::to("/login?error=server-error"),
        Err(_) => return Redirect::to("/login?error=server-error"),
    };

    // TODO: replace with real hash verification once passwords are hashed (e.g. argon2::verify)
    if member.password_hash != validated.password {
        return Redirect::to("/login?error=invalid-credentials");
    }

    // TODO add session thingy — log_member_in(&context, member.id).await, set cookie, redirect to /dashboard
    Redirect::to("/dashboard")
}