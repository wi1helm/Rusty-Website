use askama_web::__askama_web_impl::axum_core_0_5::IntoResponse;
use axum::extract::State;
use axum::Form;
use axum::response::Redirect;
use serde::Deserialize;
use crate::api::signup::{SignupRequest, SignupValidationError};
use crate::api::validation::is_valid_name;
use crate::auth::CurrentUser;
use crate::Context;

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
    //TODO
    // 1. Find the organization's profile.
    // 2. Determine the next version number.
    // 3. Create a new profile version.
    // 4. Make that version the current version.
    // We'll use these once we implement the database update.
    let _ = context;
    let _ = user;
    let _ = validated;

    Redirect::to("/profile").into_response()
}

pub async fn publish(State(context): State<Context>, CurrentUser(user): CurrentUser) -> impl IntoResponse {
    //TODO
    // 1. Find the organization's profile.
    // 2. Get the current version ID.
    // 3. Set the published version to the current version.
    //
    // Publishing does not create a new version.
    // It simply makes the current version visible publicly.

    // We'll use these once we implement the database update.
    let _ = context;
    let _ = user;

    Redirect::to("/profile").into_response()
}