use askama::Template;
use askama_web::WebTemplate;

#[derive(Template, WebTemplate)]
#[template(path = "public/creation.html")]
pub struct CreationTemplate {
    pub app_name: &'static str,
    pub year: u16,
}
// Todo add posiblity to input or choose slug.
pub async fn get_page() -> CreationTemplate {
    CreationTemplate {
        app_name: crate::config::APP_NAME,
        year: crate::config::APP_YEAR,
    }
}
