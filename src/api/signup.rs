// routes/pages/signup.rs
use serde::Deserialize;
use axum::{extract::State, Form};
use axum::response::Redirect;
use crate::{Context};
use crate::api::validation::{ is_valid_email, is_valid_name, is_valid_password};
use crate::auth::AuthenticationError;
use crate::storage::database::{DatabaseError, NewMember, NewOrganization, OrganizationRoles};

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
pub async fn signup(State(context): State<Context>, Form(input): Form<SignupRequest>) -> impl axum::response::IntoResponse {
    let validated = match (SignupValidator { input: &input }).validate() {
        Ok(v) => v,
        Err(SignupValidationError::InvalidEmail) => return Redirect::to("/signup?error=invalid-email"),
        Err(SignupValidationError::InvalidPassword) => return Redirect::to("/signup?error=invalid-password"),
        Err(SignupValidationError::InvalidName) => return Redirect::to("/signup?error=invalid-name"),
        Err(SignupValidationError::PasswordMismatch) => return Redirect::to("/signup?error=passwords-mismatch")
    };

    let organization = NewOrganization {
        name: validated.name,
    };

    let owner = NewMember {
        email: validated.email,
        password_hash: validated.password, // TODO: hash before storing
        role: OrganizationRoles::Owner,
    };

    let (organization, owner_id) = match context.database.create_organization_with_owner(organization, owner).await {
        Ok(result) => result,

        Err(DatabaseError::MemberAlreadyExists) => return Redirect::to("/signup?error=email-taken"),
        Err(DatabaseError::Sqlx(_)) => return Redirect::to("/signup?error=server-error")
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
    match context.authenticator.login_member(owner_id, organization.id).await {
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
        // Session(_) means we dont care what specific session error happened just that we could not
        // establish a session (for example, Redis could be down etc.)
        Err(AuthenticationError::Session(_)) => {
            // The account exists, but we could not create the session.
            Redirect::to("/login?error=account-created")
        }

        Err(_) => {
            // Unexpected authentication error.
            Redirect::to("/signup?error=server-error")
        }
    }

}