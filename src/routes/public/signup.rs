use askama::Template;
use askama_web::WebTemplate;

#[derive(Template, WebTemplate)]
#[template(path = "public/signup.html")]
pub struct SignupTemplate {
    pub app_name: &'static str,
    pub year: u16,
}

pub async fn signup() -> SignupTemplate {
    SignupTemplate {
        app_name: crate::config::APP_NAME,
        year: crate::config::APP_YEAR,
    }
}
