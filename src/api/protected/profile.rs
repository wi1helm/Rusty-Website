use askama_web::__askama_web_impl::axum_core_0_5::IntoResponse;
use axum::extract::State;
use axum::Json;
use axum::http::StatusCode;
use axum::response::Redirect;
use serde::Deserialize;

use crate::api::protected::get_profile;
use crate::api::validation::{is_valid_name, is_valid_url};
use crate::auth::CurrentUser;
use crate::storage::database::{DatabaseProfile, NewProfileLink, UpdateProfile};
use crate::Context;

#[derive(Debug)]
pub enum UpdateRequestValidationError {
    InvalidDisplayName,
    InvalidAbout,
    InvalidLink,
}

#[derive(Debug, Deserialize)]
pub struct UpdateRequest {
    pub display_name: String,
    pub about: String,
    pub links: Vec<ProfileLink>,
}

#[derive(Debug, Deserialize)]
pub struct ProfileLink {
    pub label: String,
    pub url: String,
}
#[derive(PartialEq)]
pub struct ValidatedUpdateRequest {
    pub display_name: String,
    pub about: String,
    pub links: Vec<ValidatedLink>,
}
#[derive(PartialEq)]
pub struct ValidatedLink {
    pub label: String,
    pub url: String,
}

impl UpdateRequest {
    fn validate(self) -> Result<ValidatedUpdateRequest, UpdateRequestValidationError> {
        if !is_valid_name(&self.display_name) {
            return Err(UpdateRequestValidationError::InvalidDisplayName);
        }
        //TODO add this check inside a function in validation file
        if self.about.len() > 5000 {
            return Err(UpdateRequestValidationError::InvalidAbout);
        }

        let mut links = Vec::with_capacity(self.links.len());

        for link in self.links {
            if !is_valid_name(&link.label) || !is_valid_url(&link.url) {
                return Err(UpdateRequestValidationError::InvalidLink);
            }

            links.push(ValidatedLink {
                label: link.label,
                url: link.url,
            });
        }

        Ok(ValidatedUpdateRequest {
            display_name: self.display_name,
            about: self.about,
            links,
        })
    }
}

impl PartialEq<ValidatedUpdateRequest> for DatabaseProfile {
    fn eq(&self, other: &ValidatedUpdateRequest) -> bool {
        self.display_name == other.display_name
            && self.about == other.about
            && self.links.len() == other.links.len()
            && self.links.iter().zip(&other.links).all(|(old, new)| {
                old.label == new.label
                && old.url == new.url
                //TODO add position thingy
            })
    }
}

pub async fn update(State(context): State<Context>, CurrentUser(user): CurrentUser, Json(input): Json<UpdateRequest>, ) -> impl IntoResponse {
    let validated = match input.validate() {
        Ok(validated) => validated,
        Err(UpdateRequestValidationError::InvalidDisplayName) => {
            return Redirect::to("/profile?error=invalid-display-name").into_response();
        }
        Err(UpdateRequestValidationError::InvalidAbout) => {
            return Redirect::to("/profile?error=invalid-about").into_response();
        }
        Err(UpdateRequestValidationError::InvalidLink) => {
            return Redirect::to("/profile?error=invalid-link").into_response();
        }
    };
    //TODo use the old profile to check if we actually need to update this profile.
    let old_profile = match get_profile(&context, user.organization.id).await {
        Ok(profile) => profile,
        Err(status) => return status.into_response(),
    };
    // If nothing changed from whats already in db disregard this request
    if old_profile == validated {
        return Redirect::to("/profile").into_response();
    }

    let new_links = validated
        .links
        .into_iter()
        .enumerate()
        .map(|(position, link)| NewProfileLink {
            label: link.label,
            url: link.url,
            position: position as i32,
        })
        .collect();

    let new_profile = UpdateProfile {
        display_name: validated.display_name,
        about: validated.about,
    };
    if let Err(error) = context.database.update_profile(user.organization.id, user.member_id, new_profile).await {
        tracing::error!(?error,profile_id = old_profile.id, "failed to update profile");
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    }
    if let Err(error) = context.database.update_profile_links(user.organization.id, new_links).await {
        tracing::error!(?error,profile_id = old_profile.id, "failed to update profile links");
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    }

    Redirect::to("/profile").into_response()
}
