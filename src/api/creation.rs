// routes/pages/creation
//! API endpoints for organization creation and signup.
//!
//! TODO: API handlers currently return `Redirect::to("/path?error=...")` for
//! every error case. This couples API endpoints to frontend routing and makes
//! the API unusable as a real REST API. For API routes, return proper JSON
//! error responses with appropriate HTTP status codes (e.g., 400 for validation
//! errors, 409 for conflicts, 500 for server errors). Reserve redirects for
//! HTML page routes. This separates concerns and makes the API testable with
//! `curl` or automated tests.
//!
//! To fix: Add `#[derive(Serialize)]` to `CreationValidationError` and return
//! JSON responses like `(StatusCode::BAD_REQUEST, Json(error))` instead of
//! redirects. Consider creating a shared error response type in a common module.
//! Search terms: "Axum JSON error responses", "REST API error handling Rust",
//! "separate API from HTML routes Axum", "problem_details RFC 7807 Rust".

use crate::api::validation::{is_valid_email, is_valid_name, is_valid_password};
use crate::auth::AuthenticationError;
use crate::auth::Authenticator;
use crate::auth::password::hash_password;
use crate::config::CookieConfig;
use crate::routes::redirect_with_session_cookie;
use crate::storage::database::{
    CreateOrganizationError, NewMember, NewOrganization, OrganizationRepository,
    OrganizationRole,
};
use axum::response::{IntoResponse, Redirect};
use axum::{Form, extract::State};
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

        let validated = ValidatedCreation {
            name: self.name.clone(),
            email: self.email.clone(),
            password: self.password.clone(),
        };
        Ok(validated)
    }
}
// TODO(AUDIT): [MEDIUM] Limit automated organization creation (AUDIT-002)
// Problem: Public signup has no rate limit or anti-automation control and creates persistent records.
// Impact: Automated signups can consume database and password-hashing resources.
// Suggested fix: Apply shared signup throttles and abuse controls before creating records.
// Reference: POST /api/organizations/create; signup.
pub async fn signup(State(organizations): State<Arc<dyn OrganizationRepository>>, State(authenticator): State<Arc<Authenticator>>, State(cookies): State<CookieConfig>, Form(input): Form<CreationRequest>, ) -> impl IntoResponse {
    let validated = match input.validate() {
        Ok(v) => v,
        Err(CreationValidationError::InvalidEmail) => { return Redirect::to("/create?error=invalid-email").into_response(); }
        Err(CreationValidationError::InvalidPassword) => { return Redirect::to("/create?error=invalid-password").into_response(); }
        Err(CreationValidationError::InvalidName) => { return Redirect::to("/create?error=invalid-name").into_response(); }
        Err(CreationValidationError::PasswordMismatch) => { return Redirect::to("/create?error=passwords-mismatch").into_response(); }
    };

    let slug = create_slug(&validated.name);
    if crate::routes::is_reserved_organization_slug(&slug) {
        return Redirect::to("/create?error=invalid-slug").into_response();
    }

    let organization = NewOrganization {
        name: validated.name,
        slug,
    };

    let password_hash = match hash_password(validated.password).await {
        Ok(password_hash) => password_hash,
        Err(error) => {
            tracing::error!(?error, "failed to hash signup password");
            return Redirect::to("/create?error=server-error").into_response();
        }
    };

    let owner = NewMember {
        email: validated.email,
        password_hash,
        role: OrganizationRole::Owner,
    };

    let (_organization_id, owner_id) = match organizations.create_organization_with_owner_and_profile(organization, owner).await {
        Ok(result) => result,
        Err(error) => { return Redirect::to(signup_error_redirect_location(error)).into_response(); }
    };

    // The organization and owner member have now been successfully created
    // in the database. We now create an authenticated session for that member.
    //
    // This is intentionally a separate step from account creation:
    // - PostgreSQL is responsible for creating and storing the account.
    // - The authenticator is responsible for establishing the logged-in state.
    // - The session is stored in Cache(Redis as of writing) and represented to the
    //   browser by a session cookie.
    //
    // If session creation fails, the account has still been created successfully.
    // We should NOT try to delete/roll back the account just because Redis failed.
    // The user can simply be sent to the login page and log in normally later.
    //
    // `Ok(session)` means authentication/session creation succeeded. The session
    // will later be put into a cookie and the user redirected to the dashboard.
    //
    // `AuthenticationError::Session(_)` means the account exists, but we could
    // not create its session (for example, Redis could be unavailable).
    //
    // The final `Err(_)` catches any other authentication error. Most of these
    // should not normally be possible here because we are logging in a member
    // by ID that was just created, rather than checking credentials again.
    // In theory the database could be unavailable directly after creating the org + owner
    // and login could therefor not access the database(password hash)
    // todo handle all of the rest Authentication errors and remove Err(_)
    match authenticator.login_member(owner_id).await {
        Ok(session) => redirect_with_session_cookie("/dashboard", &session.id, &cookies),
        // Session(_) means we dont care what specific session error happened just that we could not
        // establish a session (for example, Redis could be down etc.)
        Err(AuthenticationError::Session(_)) => {
            //TODO add logging for bug / bad errors such as serilization
            //The session ID is effectively a bearer credential. Someone who obtains a valid session ID may be able to authenticate as that user.
            // IMPORTANT TO NOT LOG the id, application logs could potentially be access
            // The account exists, but we could not create the session.
            Redirect::to("/login?error=account-created").into_response()
        }

        Err(_) => {
            // Unexpected authentication error.
            Redirect::to("/create?error=server-error").into_response()
        }
    }
}

fn signup_error_redirect_location(error: CreateOrganizationError) -> &'static str {
    match error {
        CreateOrganizationError::OrganizationSlugAlreadyExists => "/create?error=invalid-slug",
        CreateOrganizationError::MemberAlreadyExists => "/create?error=signup-failed",
        CreateOrganizationError::Database(_) => "/create?error=server-error",
    }
}

/// Generates the slug for any company name used on signup
///TODO for now if the slug is taken we reject the signup
/// later on add ways to generate new unique slug instead like slug-1, slug-2!!
// TODO handle non ascii char better then replacing with - "w-jerrest-l-ab" is not a nice slug fro "W Jerrestål AB"
fn create_slug(name: &str) -> String {
    name.to_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect::<String>()
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
        for name in [
            "Login",
            "Create",
            "Organization",
            "Dashboard",
            "Profile",
            "API",
            "Images",
            "Assets",
        ] {
            assert!(
                crate::routes::is_reserved_organization_slug(&create_slug(name)),
                "{name} should not produce a usable organization slug"
            );
        }
    }

    #[test]
    fn allows_slugs_that_do_not_match_application_routes() {
        assert!(!crate::routes::is_reserved_organization_slug(&create_slug("Acme")));
    }

    #[test]
    fn duplicate_email_uses_a_neutral_signup_error() {
        let location = signup_error_redirect_location(CreateOrganizationError::MemberAlreadyExists);

        assert_eq!(location, "/create?error=signup-failed");
        assert!(!location.contains("email"));
    }

    #[test]
    fn empty_name_produces_empty_slug() {
        assert_eq!(create_slug(""), "");
    }

    fn valid_creation_request() -> CreationRequest {
        CreationRequest {
            name: "Example Organization".to_owned(),
            email: "owner@example.com".to_owned(),
            password: "long-enough-password".to_owned(),
            confirm: "long-enough-password".to_owned(),
        }
    }

    #[test]
    fn validates_a_well_formed_creation_request() {
        let validated = valid_creation_request()
            .validate()
            .expect("valid signup fields should pass validation");

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
