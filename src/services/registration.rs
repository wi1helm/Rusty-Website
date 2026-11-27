use crate::auth::password::hash_password;
use crate::auth::session::Session;
use crate::auth::{AuthenticationError, Authenticator};
use crate::storage::database::{CreateOrganizationError, NewMember, NewOrganization, OrganizationRepository, OrganizationRole};

pub enum RegistrationError {
    PasswordHash(String),
    Organization(CreateOrganizationError),
    AccountCreatedWithoutSession,
    Authentication(AuthenticationError),
}

pub async fn create_organization(
    organizations: &dyn OrganizationRepository, authenticator: &Authenticator, organization: NewOrganization, email: String, password: String,
) -> Result<Session, RegistrationError> {
    let password_hash = hash_password(password).await.map_err(|error| RegistrationError::PasswordHash(error.to_string()))?;

    let owner = NewMember { email, password_hash, role: OrganizationRole::Owner };
    let (_, owner_id) = organizations.create_organization_with_owner_and_profile(organization, owner).await.map_err(RegistrationError::Organization)?;

    match authenticator.login_member(owner_id).await {
        Ok(session) => Ok(session),
        Err(AuthenticationError::Session(_)) => Err(RegistrationError::AccountCreatedWithoutSession),
        Err(error) => Err(RegistrationError::Authentication(error)),
    }
}
