use askama::Template;
use askama_web::WebTemplate;
use axum::{
    extract::State,
    response::{IntoResponse, Response},
};
use axum::response::Html;
use crate::{auth::CurrentUser, Context};
use crate::config::{APP_NAME, APP_YEAR};
#[derive(Template, WebTemplate)]
#[template(path = "protected/profile.html")]
pub struct ProfileTemplate {
    pub app_name: &'static str,
    pub year: u16,
    pub organization_name: String,
    pub organization_profile: Profile,
}

pub struct Profile {
    pub display_name: String,
    pub slug: String,

    pub about: String,

    pub locations: Vec<Location>,
    pub links: Vec<ExternalLink>,

    pub logo: Option<Image>,
    pub banner: Option<Image>,
}

pub struct Location {
    pub city: String,
    pub country: String,
}

pub struct ExternalLink {
    pub label: String,
    pub url: String,
}

pub struct Image {
    pub url: String,
    pub alt: String,
}


pub async fn get_page(State(context): State<Context>, CurrentUser(user): CurrentUser) -> Response {
    let organization_id = user.organization.id;

    // TODO: Load the organization profile using `organization_id`.
    //
    // The organization itself does not need to be loaded again because
    // `CurrentUser` already contains the authenticated organization's
    // identity and name.

    // TODO: Replace these placeholder values with data loaded from the
    // organization profile.
    let profile = Profile {
        display_name: user.organization.name.clone(),

        slug: "acme-robotics".to_string(),

        about: "Acme Robotics is a technology company focused on building \
                autonomous robotic systems for industrial environments."
            .to_string(),

        locations: vec![
            Location {
                city: "Stockholm".to_string(),
                country: "Sweden".to_string(),
            },
        ],

        links: vec![
            ExternalLink {
                label: "Website".to_string(),
                url: "https://example.com".to_string(),
            },
            ExternalLink {
                label: "GitHub".to_string(),
                url: "https://github.com/example".to_string(),
            },
        ],

        logo: Some(Image {
            url: "/assets/test/logo.png".to_string(),
            alt: format!("{} logo", user.organization.name),
        }),

        banner: Some(Image {
            url: "/assets/test/banner.jpg".to_string(),
            alt: format!("{} banner", user.organization.name),
        }),
    };

    ProfileTemplate {
        app_name: APP_NAME,
        year: APP_YEAR,
        organization_name: user.organization.name,
        organization_profile: profile,
    }.into_response()
}