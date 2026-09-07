use askama_web::__askama_web_impl::axum_core_0_5::IntoResponse;
use axum::extract::State;
use axum::Form;
use axum::http::StatusCode;
use axum::response::Redirect;
use serde::Deserialize;
use crate::api::validation::is_valid_name;
use crate::auth::CurrentUser;
use crate::Context;
use crate::storage::database::{DatabaseProfile, NewProfileVersion, PublishProfileError};

#[derive(Debug)]
pub enum EditValidationError {
    InvalidDisplayName,
    InvalidAbout,
}

#[derive(Debug, Deserialize)]
pub struct EditRequest {
    pub display_name: String,
    pub about: String,
}

pub struct ValidatedEdit {
    pub display_name: String,
    pub about: String,
}

struct EditValidator<'a> {
    input: &'a EditRequest,
}

impl EditValidator<'_> {
    fn validate(&self) -> Result<ValidatedEdit, EditValidationError> {
        if !is_valid_name(&self.input.display_name) {
            return Err(EditValidationError::InvalidDisplayName);
        }
        // TODO add a section for about in validation.rs
        // You can add whatever rules you decide for `about`.
        if self.input.about.len() > 5000 {
            return Err(EditValidationError::InvalidAbout);
        }

        Ok(ValidatedEdit {
            display_name: self.input.display_name.clone(),
            about: self.input.about.clone(),
        })
    }
}
pub async fn edit(State(context): State<Context>, CurrentUser(user): CurrentUser, Form(input): Form<EditRequest>) -> impl IntoResponse {
    let validated = match (EditValidator { input: &input }).validate() {
        Ok(v) => v,
        Err(EditValidationError::InvalidDisplayName) => {
            return Redirect::to("/profile?error=invalid-display-name").into_response();
        }
        Err(EditValidationError::InvalidAbout) => {
            return Redirect::to("/profile?error=invalid-about").into_response();
        }
    };

    let profile = match get_profile(&context, user.organization.id).await {
        Ok(profile) => profile,
        Err(status) => return status.into_response(),
    };

    let current_version_id = match profile.current_version_id {
        Some(id) => id,
        None => {
            tracing::error!(profile_id = profile.id, organization_id = user.organization.id, "profile has no current version");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    let current_version =
        match context.database.find_profile_version_by_version_id(current_version_id).await {
            Ok(Some(version)) => version,

            Ok(None) => {
                tracing::error!(profile_id = profile.id, current_version_id, "profile references a version that does not exist");
                return StatusCode::INTERNAL_SERVER_ERROR.into_response();
            }

            Err(error) => {
                tracing::error!(?error, profile_id = profile.id, current_version_id, "failed to load current profile version");
                return StatusCode::INTERNAL_SERVER_ERROR.into_response();
            }
        };

    let next_version = current_version.version + 1;

    let new_profile_version = NewProfileVersion {
        profile_id: profile.id,
        version: next_version,
        display_name: validated.display_name,
        about: validated.about,
        created_by_member_id: user.member_id,
    };

    let _version_id = match context.database.create_new_profile_version(new_profile_version).await {
        Ok(id) => id,
        Err(error) => {
            tracing::error!(?error, profile_id = profile.id, "failed to create new profile version");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };
    //TODO something i toughtr of cant this redirect still use old info, if we somewhere cache data in future instead of fetching diretly.
    Redirect::to("/profile").into_response()
}

pub async fn publish(State(context): State<Context>, CurrentUser(user): CurrentUser) -> impl IntoResponse {
    // Publishing does not create a new version.
    // It simply makes the current version visible publicly.

    let profile = match get_profile(&context, user.organization.id).await {
        Ok(profile) => profile,
        Err(status) => return status.into_response(),
    };

    match context.database.publish_current_profile_version(profile.id).await {
        Ok(()) => {}

        Err(PublishProfileError::ProfileNotFound) => {
            tracing::error!(
                profile_id = profile.id,
                organization_id = user.organization.id,
                "profile disappeared while publishing"
            );

            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }

        Err(PublishProfileError::ProfileHasNoCurrentVersion) => {
            tracing::error!(
                profile_id = profile.id,
                organization_id = user.organization.id,
                "profile has no current version"
            );

            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }

        Err(PublishProfileError::Database(error)) => {
            tracing::error!(
                ?error,
                profile_id = profile.id,
                organization_id = user.organization.id,
                "failed to publish profile"
            );

            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    }

    Redirect::to("/profile").into_response()
}


async fn get_profile(context: &Context, organization_id: i64, ) -> Result<DatabaseProfile, StatusCode> {
    match context.database.find_profile_by_organization_id(organization_id).await {
        Ok(Some(profile)) => Ok(profile),

        Ok(None) => {
            tracing::error!(organization_id, "organization exists without a profile");
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }

        Err(error) => {
            tracing::error!(?error,organization_id,"failed to load organization profile");
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}