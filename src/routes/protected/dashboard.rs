use axum::{
    extract::State,
    response::Html,
};

use crate::{
    auth::AuthenticatedSession,
    Context,
};

/// Minimal HTML-escaping for a single value.
fn escape_html(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

/// Substitutes `{{PLACEHOLDER}}` tokens in an HTML template with raw
/// (unescaped) values, escaping each value before it goes in.
///
/// This is the ONLY function that should be used to interpolate
/// user-controlled data into an HTML template. Because escaping happens
/// here rather than at each call site, there's no path where a handler
/// can forget to call `escape_html` -- passing the raw value in is
/// itself always safe.
///
/// `replacements` is a list of (placeholder, raw_value) pairs, e.g.
/// `&[("{{ORGANIZATION_NAME}}", &organization.name)]`.
fn render_template(template: &str, replacements: &[(&str, &str)]) -> String {
    let mut output = template.to_string();

    for (placeholder, raw_value) in replacements {
        output = output.replace(placeholder, &escape_html(raw_value));
    }

    output
}
//TODO: this hand-rolled placeholder substitution works but doesn't scale
// every new page means another render_template() call site to remember,
// and it has none of the compile-time checking a real template engine gives
// you (e.g. catching a typo'd {{PLACEHOLDER}} or a template/struct mismatch
// at build time instead of at runtime). Once we're templating more than a
// couple of pages, switch to askama (compiles .html templates against a
// Rust struct, escapes by default) or tera (more dynamic/runtime, still
// escapes by default). Either would replace escape_html + render_template
// entirely.
pub async fn dashboard(State(context): State<Context>, AuthenticatedSession(session): AuthenticatedSession, ) -> Html<String> {
    let organization = match context.database.find_organization_by_id(session.organization_id).await {
        Ok(Some(organization)) => organization,

        Ok(None) => {
            // TODO: Decide what should happen if an authenticated
            // session refers to an organization that no longer exists.
            return Html("<h1>Organization not found</h1>".to_string());
        }

        Err(error) => {
            tracing::error!(?error,organization_id = session.organization_id,"failed to load organization for dashboard");

            return Html("<h1>Something went wrong</h1>".to_string());
        }
    };

    let template = include_str!("../../../frontend/dashboard/index.html");

    Html(render_template(
        template,
        &[("{{ORGANIZATION_NAME}}", &organization.name)],
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escape_html_escapes_all_special_characters() {
        assert_eq!(
            escape_html("<script>&\"'</script>"),
            "&lt;script&gt;&amp;&quot;&#39;&lt;/script&gt;"
        );
    }

    #[test]
    fn escape_html_leaves_plain_text_unchanged() {
        assert_eq!(escape_html("Acme Inc."), "Acme Inc.");
    }

    #[test]
    fn render_template_substitutes_and_escapes() {
        let template = "<h1>{{ORGANIZATION_NAME}}</h1>";
        let output = render_template(
            template,
            &[("{{ORGANIZATION_NAME}}", "<script>alert(1)</script>")],
        );

        assert_eq!(
            output,
            "<h1>&lt;script&gt;alert(1)&lt;/script&gt;</h1>"
        );
    }

    #[test]
    fn render_template_supports_multiple_placeholders() {
        let template = "{{A}} and {{B}}";
        let output = render_template(
            template,
            &[("{{A}}", "<b>x</b>"), ("{{B}}", "<i>y</i>")],
        );

        assert_eq!(output, "&lt;b&gt;x&lt;/b&gt; and &lt;i&gt;y&lt;/i&gt;");
    }
}