// routes/pages/signup.rs
use serde::Deserialize;
use axum::{extract::State, Form};
use axum::response::Redirect;
use crate::{Context, api::validation::{ValidationError, is_valid_email}};
use crate::api::validation::{is_valid_name, is_valid_password};
use crate::database::{DatabaseError, NewMember, NewOrganization, OrganizationRoles};

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
    fn validate(&self) -> Result<ValidatedSignup, ValidationError> {
        if !is_valid_name(&self.input.name) {
            return Err(ValidationError::InvalidName);
        }
        if !is_valid_email(&self.input.email) {
            return Err(ValidationError::InvalidEmail);
        }
        if !is_valid_password(&self.input.password) || !is_valid_password(&self.input.confirm) {
            return Err(ValidationError::InvalidPassword);
        }
        if self.input.password != self.input.confirm {
            return Err(ValidationError::PasswordMismatch);
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
    // Check so its valid
    let validated = match (SignupValidator { input: &input }).validate() {
        Ok(v) => v,
        Err(ValidationError::InvalidEmail) => return Redirect::to("/signup?error=invalid-email"),
        Err(ValidationError::InvalidPassword) => return Redirect::to("/signup?error=invalid-password"),
        Err(ValidationError::InvalidName) => return Redirect::to("/signup?error=invalid-name"),
        Err(ValidationError::PasswordMismatch) => return Redirect::to("/signup?error=passwords-mismatch"),
    };
    // Create new organization
    let organization = NewOrganization {
        name: validated.name,
    };
    // Create new member with owner role
    let owner = NewMember {
        email: validated.email,
        password_hash: validated.password, // TODO: hash before storing
        role: OrganizationRoles::Owner,
    };
    // Run the database function and handle errors
    match context.database.create_organization_with_owner(organization, owner).await {
        Ok((org,owner_id)) => {
            Redirect::to("/dashboard")
        }, //TODO add session thingy to get data from. Idea is to make it run login member from login api receve cookie token for seesion and redirect to dashboard
        Err(DatabaseError::MemberAlreadyExists) => Redirect::to("/signup?error=email-taken"),
        Err(DatabaseError::Sqlx(_)) => Redirect::to("/signup?error=server-error"),
    }
}