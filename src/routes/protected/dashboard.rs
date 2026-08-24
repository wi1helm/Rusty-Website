use askama::Template;
use askama_web::WebTemplate;
use axum::{
    extract::State,
    response::{IntoResponse, Response},
};
use axum::response::Html;
use crate::{auth::AuthenticatedSession, Context};
use crate::config::{APP_NAME, APP_YEAR};

#[derive(Template, WebTemplate)]
#[template(path = "protected/dashboard.html")]
struct DashboardTemplate {
    app_name: &'static str,
    year: u16,
    organization_name: String,
}

pub async fn dashboard(State(context): State<Context>, AuthenticatedSession(session): AuthenticatedSession, ) -> Response {
    let organization = match context.database.find_organization_by_id(session.organization_id).await {
        Ok(Some(organization)) => organization,

        Ok(None) => {
            // TODO: Decide what should happen if an authenticated
            // session refers to an organization that no longer exists.
            return Html("<h1>Organization not found</h1>").into_response();
        }

        Err(error) => {
            tracing::error!(?error,organization_id = session.organization_id,"failed to load organization for dashboard");

            return Html("<h1>Something went wrong</h1>").into_response();
        }
    };

    DashboardTemplate {
        app_name: APP_NAME,
        year: APP_YEAR,
        organization_name: organization.name,
    }.into_response()
}
