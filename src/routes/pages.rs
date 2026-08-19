use axum::response::Html;

pub async fn main() -> Html<&'static str> {
    Html(include_str!("../../frontend/main/index.html"))
}

pub async fn organization() -> Html<&'static str> {
    Html(include_str!("../../frontend/organization/index.html"))
}
