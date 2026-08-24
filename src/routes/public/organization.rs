use askama::Template;
use askama_web::WebTemplate;

#[derive(Template, WebTemplate)]
#[template(path = "public/organization.html")]
pub struct OrganizationTemplate {
    pub app_name: &'static str,
    pub year: u16,
}

pub async fn get_page() -> OrganizationTemplate {
    OrganizationTemplate {
        app_name: crate::config::APP_NAME,
        year: crate::config::APP_YEAR,
    }
}
