use askama_web::__askama_web_impl::axum_core_0_5::IntoResponse;
use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::Redirect;
use serde::Deserialize;
use std::sync::Arc;

use crate::api::protected::get_profile;
use crate::api::validation::{is_valid_about, is_valid_name, is_valid_url};
use crate::auth::CurrentUser;
use crate::storage::database::{DatabaseProfile, NewProfileLink, ProfileRepository, UpdateProfile};

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
        if !is_valid_about(&self.about) {
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
                old.label == new.label && old.url == new.url
                //TODO add position thingy
            })
    }
}

pub async fn update(State(profiles): State<Arc<dyn ProfileRepository>>, CurrentUser(user): CurrentUser, Json(input): Json<UpdateRequest>, ) -> impl IntoResponse {
    let validated = match input.validate() {
        Ok(validated) => validated,
        Err(UpdateRequestValidationError::InvalidDisplayName) => { return Redirect::to("/profile?error=invalid-display-name").into_response(); }
        Err(UpdateRequestValidationError::InvalidAbout) => { return Redirect::to("/profile?error=invalid-about").into_response(); }
        Err(UpdateRequestValidationError::InvalidLink) => { return Redirect::to("/profile?error=invalid-link").into_response(); }
    };
    //TODo use the old profile to check if we actually need to update this profile.
    let old_profile = match get_profile(profiles.as_ref(), user.organization.id).await {
        Ok(profile) => profile,
        Err(status) => return status.into_response(),
    };
    // If nothing changed from what's already in db disregard this request
    if old_profile == validated { return Redirect::to("/profile").into_response(); }

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
    // TODO(AUDIT): [MEDIUM] Make profile fields and links update atomically (AUDIT-005)
    // Problem: Profile fields commit before links are written in a separate transaction.
    // Impact: A link-write failure leaves a partially updated public profile while returning an error.
    // Suggested fix: Move both repository writes into one database transaction.
    // Reference: POST /api/profile/update; ProfileRepository::update_profile and update_profile_links.
    if let Err(error) = profiles.update_profile(user.organization.id, user.member_id, new_profile).await {
        tracing::error!(?error,profile_id = old_profile.id,"failed to update profile");
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    }
    if let Err(error) = profiles.update_profile_links(user.organization.id, new_links).await {
        tracing::error!(?error,profile_id = old_profile.id,"failed to update profile links");
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    }

    Redirect::to("/profile").into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_update_request() -> UpdateRequest {
        UpdateRequest {
            display_name: "Example Organization".to_owned(),
            about: "About this organization".to_owned(),
            links: vec![ProfileLink {
                label: "Website".to_owned(),
                url: "https://example.com".to_owned(),
            }],
        }
    }

    #[test]
    fn accepts_valid_profile_update() {
        let validated = valid_update_request()
            .validate()
            .expect("valid profile update should pass validation");

        assert_eq!(validated.display_name, "Example Organization");
        assert_eq!(validated.about, "About this organization");
        assert_eq!(validated.links.len(), 1);
        assert_eq!(validated.links[0].label, "Website");
        assert_eq!(validated.links[0].url, "https://example.com");
    }

    #[test]
    fn accepts_about_at_5000_unicode_characters() {
        let mut request = valid_update_request();
        request.about = "é".repeat(5000);

        assert!(request.validate().is_ok());
    }

    #[test]
    fn rejects_invalid_display_name() {
        let mut request = valid_update_request();
        request.display_name = "<script>".to_owned();

        assert!(matches!(request.validate(), Err(UpdateRequestValidationError::InvalidDisplayName)));
    }

    #[test]
    fn rejects_about_above_5000_unicode_characters() {
        let mut request = valid_update_request();
        request.about = "é".repeat(5001);

        assert!(matches!(request.validate(), Err(UpdateRequestValidationError::InvalidAbout)));
    }

    #[test]
    fn rejects_invalid_profile_links() {
        let mut request = valid_update_request();
        request.links[0].url = "javascript:alert(1)".to_owned();

        assert!(matches!(request.validate(), Err(UpdateRequestValidationError::InvalidLink)));
    }
}
