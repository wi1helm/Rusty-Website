use askama::Template;
use askama_web::WebTemplate;

#[derive(Template, WebTemplate)]
#[template(path = "public/login.html")]
pub struct LoginTemplate {
    pub app_name: &'static str,
    pub year: u16,
}

pub async fn get_page() -> LoginTemplate {
    LoginTemplate {
        app_name: crate::config::APP_NAME,
        year: crate::config::APP_YEAR,
    }
}
