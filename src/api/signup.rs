// routes/pages/signup.rs
use serde::Deserialize;
use axum::{extract::State, Form};
use axum::response::{IntoResponse, Redirect};
use crate::{Context};
use crate::api::validation::{ is_valid_email, is_valid_name, is_valid_password};
use crate::auth::AuthenticationError;
use crate::routes::redirect_with_session_cookie;
use crate::storage::database::{CreateOrganizationError, NewMember, NewOrganization, OrganizationRoles};

#[derive(Debug)]
pub enum SignupValidationError {
    InvalidName,
    InvalidEmail,
    InvalidPassword,
    PasswordMismatch,
}


// This represents that the application does not deem trustworthy.
// #derive means this struct is automaticly creates from form data.
#[derive(Deserialize)]
pub struct SignupRequest {
    pub name: String,
    pub email: String,
    pub password: String,
    pub confirm: String,
}
// The output, removed confirm as if valid password and confirm is the same.
// This represents data the application deems trustworthy
pub struct ValidatedSignup {
    pub name: String,
    pub email: String,
    pub password: String,
}

struct SignupValidator<'a> {
    input: &'a SignupRequest,
}

impl SignupValidator<'_> {
    fn validate(&self) -> Result<ValidatedSignup, SignupValidationError> {
        if !is_valid_name(&self.input.name) {
            return Err(SignupValidationError::InvalidName);
        }
        if !is_valid_email(&self.input.email) {
            return Err(SignupValidationError::InvalidEmail);
        }
        if !is_valid_password(&self.input.password) || !is_valid_password(&self.input.confirm) {
            return Err(SignupValidationError::InvalidPassword);
        }
        if self.input.password != self.input.confirm {
            return Err(SignupValidationError::PasswordMismatch);
        }

        let validated = ValidatedSignup {
            name: self.input.name.clone(),
            email: self.input.email.clone(),
            password: self.input.password.clone(),
        };
        Ok(validated)
    }
}
pub async fn signup(State(context): State<Context>, Form(input): Form<SignupRequest>) -> impl IntoResponse {
    let validated = match (SignupValidator { input: &input }).validate() {
        Ok(v) => v,
        Err(SignupValidationError::InvalidEmail) => return Redirect::to("/signup?error=invalid-email").into_response(),
        Err(SignupValidationError::InvalidPassword) => return Redirect::to("/signup?error=invalid-password").into_response(),
        Err(SignupValidationError::InvalidName) => return Redirect::to("/signup?error=invalid-name").into_response(),
        Err(SignupValidationError::PasswordMismatch) => return Redirect::to("/signup?error=passwords-mismatch").into_response()
    };

    let slug = create_slug(&validated.name);

    let organization = NewOrganization {
        name: validated.name,
        slug,
    };

    let owner = NewMember {
        email: validated.email,
        password_hash: validated.password, // TODO: hash before storing
        role: OrganizationRoles::Owner,
    };

    let (_organization_id, owner_id) = match context.database.create_organization_with_owner_and_profile(organization, owner).await {
        Ok(result) => result,

        Err(CreateOrganizationError::OrganizationSlugAlreadyExists) => return Redirect::to("/signup?error=invalid-slug").into_response(),
        Err(CreateOrganizationError::MemberAlreadyExists) => return Redirect::to("/signup?error=email-taken").into_response(),
        Err(CreateOrganizationError::Database(_error)) => return Redirect::to("/signup?error=server-error").into_response()
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
    match context.authenticator.login_member(owner_id).await {
        Ok(session) => {
            redirect_with_session_cookie("/dashboard", &session.id)
        }
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
            Redirect::to("/signup?error=server-error").into_response()
        }
    }

}
/// Generates the slug for any company name used on signup
///TODO for now if the slug is taken we reject the signup
/// later on add ways to generate new unique slug instead like slug-1, slug-2!!
fn create_slug(name: &str) -> String {
    name.to_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect::<String>()
}