// routes/pages/creation
//! API endpoints for organization creation and signup.
//!
use crate::api::error::{ApiSuccessResponse, api_error};
use crate::api::validation::{is_valid_email, is_valid_name, is_valid_password};
use crate::auth::Authenticator;
use crate::config::CookieConfig;
use crate::routes::{is_reserved_organization_slug, with_session_cookie};
use crate::services::registration::{self, RegistrationError};
use crate::storage::database::{CreateOrganizationError, NewOrganization, OrganizationRepository};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::{Json, extract::State};
use serde::Deserialize;
use std::sync::Arc;

#[derive(Debug)]
pub enum CreationValidationError {
    InvalidName,
    InvalidEmail,
    InvalidPassword,
    PasswordMismatch,
}

// This represents that the application does not deem trustworthy.
// #derive means this struct is automaticly creates from form data.
#[derive(Deserialize)]
pub struct CreationRequest {
    pub name: String,
    pub email: String,
    pub password: String,
    pub confirm: String,
}
// The output, removed confirm as if valid password and confirm is the same.
// This represents data the application deems trustworthy
pub struct ValidatedCreation {
    pub name: String,
    pub email: String,
    pub password: String,
}

impl CreationRequest {
    fn validate(&self) -> Result<ValidatedCreation, CreationValidationError> {
        if !is_valid_name(&self.name) {
            return Err(CreationValidationError::InvalidName);
        }
        if !is_valid_email(&self.email) {
            return Err(CreationValidationError::InvalidEmail);
        }
        if !is_valid_password(&self.password) || !is_valid_password(&self.confirm) {
            return Err(CreationValidationError::InvalidPassword);
        }
        if self.password != self.confirm {
            return Err(CreationValidationError::PasswordMismatch);
        }

        let validated = ValidatedCreation { name: self.name.clone(), email: self.email.clone(), password: self.password.clone() };
        Ok(validated)
    }
}
// TODO(AUDIT): [MEDIUM] Limit automated organization creation (AUDIT-002)
// Problem: Public signup has no rate limit or anti-automation control and creates persistent records.
// Impact: Automated signups can consume database and password-hashing resources.
// Suggested fix: Apply shared signup throttles and abuse controls before creating records.
// Reference: POST /api/organizations/create; signup.
pub async fn create(
    State(organizations): State<Arc<dyn OrganizationRepository>>, State(authenticator): State<Arc<Authenticator>>, State(cookies): State<CookieConfig>, Json(input): Json<CreationRequest>,
) -> impl IntoResponse {
    let validated = match input.validate() {
        Ok(v) => v,
        Err(CreationValidationError::InvalidEmail) => {
            return api_error(StatusCode::BAD_REQUEST, "invalid-email", "Please enter a valid email address.");
        }
        Err(CreationValidationError::InvalidPassword) => {
            return api_error(StatusCode::BAD_REQUEST, "invalid-password", "Password must be at least 8 characters long.");
        }
        Err(CreationValidationError::InvalidName) => {
            return api_error(StatusCode::BAD_REQUEST, "invalid-name", "Please enter a valid organization name.");
        }
        Err(CreationValidationError::PasswordMismatch) => {
            return api_error(StatusCode::BAD_REQUEST, "passwords-mismatch", "Passwords do not match.");
        }
    };

    let slug = create_slug(&validated.name);
    if is_reserved_organization_slug(&slug) {
        return api_error(StatusCode::BAD_REQUEST, "invalid-slug", "That organization name uses a reserved URL. Please choose another name.");
    }

    let organization = NewOrganization { name: validated.name, slug };

    let session = match registration::create_organization(organizations.as_ref(), authenticator.as_ref(), organization, validated.email, validated.password).await {
        Ok(session) => session,
        Err(RegistrationError::Organization(error)) => {
            let (status, code, message) = create_error(error);
            return api_error(status, code, message);
        }
        Err(RegistrationError::PasswordHash(error)) => {
            tracing::error!(?error, "failed to hash signup password");
            return api_error(StatusCode::INTERNAL_SERVER_ERROR, "server-error", "Something went wrong. Please try again.");
        }
        Err(RegistrationError::AccountCreatedWithoutSession) => {
            return api_error(StatusCode::SERVICE_UNAVAILABLE, "account-created", "Your account is ready, but we couldn't sign you in. Please log in.");
        }
        Err(RegistrationError::Authentication(error)) => {
            tracing::error!(?error, "failed to establish signup session");
            return api_error(StatusCode::INTERNAL_SERVER_ERROR, "server-error", "Something went wrong. Please try again.");
        }
    };

    with_session_cookie((StatusCode::CREATED, Json(ApiSuccessResponse::redirect_to("/dashboard"))).into_response(), &session.id, &cookies)
}

fn create_error(error: CreateOrganizationError) -> (StatusCode, &'static str, &'static str) {
    match error {
        CreateOrganizationError::OrganizationSlugAlreadyExists => (StatusCode::CONFLICT, "organization-exists", "An organization with that URL already exists."),
        CreateOrganizationError::MemberAlreadyExists => (
            StatusCode::CONFLICT,
            "signup-failed",
            "We couldn't complete signup. Please try again, or log in if you already have access.",
        ),
        CreateOrganizationError::Database(error) => {
            tracing::error!(?error, "database error during signup");
            (StatusCode::INTERNAL_SERVER_ERROR, "server-error", "Something went wrong. Please try again.")
        }
    }
}

/// Generates the slug for any company name used on signup
///TODO for now if the slug is taken we reject the signup
/// later on add ways to generate new unique slug instead like slug-1, slug-2!!
// TODO handle non ascii char better then replacing with - "w-jerrest-l-ab" is not a nice slug fro "W Jerrestål AB"
fn create_slug(name: &str) -> String {
    name.to_lowercase().chars().map(|c| if c.is_ascii_alphanumeric() { c } else { '-' }).collect::<String>()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creates_slug_from_simple_name() {
        assert_eq!(create_slug("Acme"), "acme");
    }

    #[test]
    fn converts_spaces_to_hyphens() {
        assert_eq!(create_slug("Acme Corporation"), "acme-corporation");
    }

    #[test]
    fn converts_special_characters_to_hyphens() {
        assert_eq!(create_slug("Acme & Sons!"), "acme---sons-");
    }

    #[test]
    fn converts_uppercase_to_lowercase() {
        assert_eq!(create_slug("ACME CORPORATION"), "acme-corporation");
    }

    #[test]
    fn non_ascii_characters_are_replaced() {
        assert_eq!(create_slug("W Jerrestål AB"), "w-jerrest-l-ab");
    }

    #[test]
    fn rejects_slugs_that_match_application_routes() {
        for name in ["Login", "Create", "Organization", "Dashboard", "Profile", "API", "Images", "Assets"] {
            assert!(crate::routes::is_reserved_organization_slug(&create_slug(name)), "{name} should not produce a usable organization slug");
        }
    }

    #[test]
    fn allows_slugs_that_do_not_match_application_routes() {
        assert!(!crate::routes::is_reserved_organization_slug(&create_slug("Acme")));
    }

    #[test]
    fn duplicate_email_uses_a_neutral_structured_signup_error() {
        let (status, code, message) = create_error(CreateOrganizationError::MemberAlreadyExists);

        assert_eq!(status, StatusCode::CONFLICT);
        assert_eq!(code, "signup-failed");
        assert!(!message.contains("email"));
    }

    #[test]
    fn empty_name_produces_empty_slug() {
        assert_eq!(create_slug(""), "");
    }

    fn valid_creation_request() -> CreationRequest {
        CreationRequest { name: "Example Organization".to_owned(), email: "owner@example.com".to_owned(), password: "long-enough-password".to_owned(), confirm: "long-enough-password".to_owned() }
    }

    #[test]
    fn validates_a_well_formed_creation_request() {
        let validated = valid_creation_request().validate().expect("valid signup fields should pass validation");

        assert_eq!(validated.name, "Example Organization");
        assert_eq!(validated.email, "owner@example.com");
        assert_eq!(validated.password, "long-enough-password");
    }

    #[test]
    fn rejects_invalid_organization_name() {
        let mut request = valid_creation_request();
        request.name = "A".to_owned();

        assert!(matches!(request.validate(), Err(CreationValidationError::InvalidName)));
    }

    #[test]
    fn rejects_invalid_email() {
        let mut request = valid_creation_request();
        request.email = "not-an-email".to_owned();

        assert!(matches!(request.validate(), Err(CreationValidationError::InvalidEmail)));
    }

    #[test]
    fn rejects_invalid_password() {
        let mut request = valid_creation_request();
        request.password = "short".to_owned();

        assert!(matches!(request.validate(), Err(CreationValidationError::InvalidPassword)));
    }

    #[test]
    fn rejects_password_confirmation_mismatch() {
        let mut request = valid_creation_request();
        request.confirm = "a-different-valid-password".to_owned();

        assert!(matches!(request.validate(), Err(CreationValidationError::PasswordMismatch)));
    }
}
