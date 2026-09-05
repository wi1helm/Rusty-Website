use askama::Template;
use askama_web::WebTemplate;
use axum::{extract::State, response::{IntoResponse, Response}};
use crate::{auth::CurrentUser, Context};
use crate::config::{APP_NAME, APP_YEAR};

#[derive(Template, WebTemplate)]
#[template(path = "protected/dashboard.html")]
struct DashboardTemplate {
    app_name: &'static str,
    year: u16,
    organization_name: String,
}

pub async fn get_page(State(context): State<Context>, CurrentUser(user): CurrentUser) -> Response {
    //TODO add and load statistics seperatly and remkae template add them.
    DashboardTemplate {
        app_name: APP_NAME,
        year: APP_YEAR,
        organization_name: user.organization.name,
    }.into_response()
}
