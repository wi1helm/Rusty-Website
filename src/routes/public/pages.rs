use axum::response::Html;

// These handlers serve the HTML files directly from the static directory.
//
// IMPORTANT:
// The paths used by `include_str!` are relative to THIS SOURCE FILE,
// not the project root. If the static directory structure changes,
// these paths must be updated.
//
// These paths are also resolved at compile time, so the files must exist
// when the application is compiled.

pub async fn main() -> Html<&'static str> {
    Html(include_str!("../../../static/main/index.html"))
}

pub async fn organization() -> Html<&'static str> {
    Html(include_str!("../../../static/organization/index.html"))
}


