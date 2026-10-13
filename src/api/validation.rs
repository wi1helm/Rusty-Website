// routes/api/validation.rs

/// Checks whether a name meets the basic requirements expected by the
/// application.
///
/// This only validates the format of the name. It does not determine
/// whether the name represents a real person.
pub fn is_valid_name(name: &str) -> bool {
    const MIN_NAME_LENGTH: usize = 2;
    const MAX_NAME_LENGTH: usize = 100;

    // Keep names within a reasonable length.
    if name.len() < MIN_NAME_LENGTH || name.len() > MAX_NAME_LENGTH {
        return false;
    }

    // Leading or trailing whitespace is not allowed.
    //
    // For example:
    // "Alice"   -> valid
    // " Alice"  -> invalid
    // "Alice "  -> invalid
    if name != name.trim() {
        return false;
    }

    // Control characters such as newlines, carriage returns, and null
    // characters are not allowed in a name.
    if name.chars().any(char::is_control) {
        return false;
    }

    // Angle brackets are not allowed in a name.
    //
    // This is NOT the app's XSS defense, every place a name is rendered
    // into HTML must escape it regardless of what's allowed here. This is
    // just data-quality: organization/member names shouldn't legitimately
    // contain markup, so we reject it early and give the user a clear
    // error instead of silently mangling it later.
    if name.contains('<') || name.contains('>') {
        return false;
    }

    true
}

/// Performs basic structural validation of an email address.
///
/// This does not verify that the email address actually exists or that
/// the user has access to it. That requires something outside of input
/// validation, such as email verification.
pub fn is_valid_email(email: &str) -> bool {
    // 254 characters is a commonly used practical maximum length
    // for an email address.
    const MAX_EMAIL_LENGTH: usize = 254;

    // The shortest email address we consider structurally valid is
    // something like "a@b".
    // NOTE: this should be discussed further
    const MIN_EMAIL_LENGTH: usize = 3;

    // Reject emails outside our allowed length range.
    if email.len() > MAX_EMAIL_LENGTH || email.len() < MIN_EMAIL_LENGTH {
        return false;
    }

    // An email address must contain exactly one '@' character.
    if email.matches('@').count() != 1 {
        return false;
    }

    // Split the email into its local part and domain.
    //
    // "user@example.com"
    //  ^^^^
    //  local
    //
    // "user@example.com"
    //      ^^^^^^^^^^^
    //      domain
    //
    // We already know there is exactly one '@', so split_once() should
    // successfully produce both parts.
    let Some((local, domain)) = email.split_once('@') else {
        return false;
    };

    // Both the local part and domain must contain at least one character.
    //
    // "@example.com" -> invalid
    // "user@"        -> invalid
    if local.is_empty() || domain.is_empty() {
        return false;
    }

    // Whitespace is not allowed anywhere in the email address.
    //
    // This also catches tabs, newlines, and other whitespace characters.
    if email.chars().any(char::is_whitespace) {
        return false;
    }

    // For this application's basic validation, require the domain to
    // contain a dot.
    //
    // "user@example.com" -> valid
    // "user@example"     -> invalid
    if !domain.contains('.') {
        return false;
    }

    // The local part cannot begin or end with a dot.
    //
    // ".user@example.com" -> invalid
    // "user.@example.com" -> invalid
    if local.starts_with('.') || local.ends_with('.') {
        return false;
    }

    // Consecutive dots are not allowed in either part.
    //
    // "user..name@example.com" -> invalid
    // "user@example..com"      -> invalid
    if local.contains("..") || domain.contains("..") {
        return false;
    }

    // NOTE: add more checks if needed later /wi1helm

    // Passing these checks only means the email has an acceptable
    // structure. It does not prove that the address exists or belongs
    // to the person submitting it.
    true
}

/// Checks whether a password meets the application's basic input
/// requirements.
///
/// This validates the password's format and length only. It does not
/// hash, verify, or otherwise determine whether the password is correct.
pub fn is_valid_password(password: &str) -> bool {
    // NOTE: i guess 8 is reasonable could be changed
    // TODO impose more strict reqs for example digits not just length for stronger passwords( maby)
    const MIN_PASSWORD_LENGTH: usize = 8;
    const MAX_PASSWORD_LENGTH: usize = 128;

    // Keep passwords within the application's accepted length range.
    if password.len() < MIN_PASSWORD_LENGTH || password.len() > MAX_PASSWORD_LENGTH {
        return false;
    }

    // Control characters such as newlines, carriage returns, and null
    // characters are not allowed.
    if password.chars().any(char::is_control) {
        return false;
    }

    true
}
/// Checks whether a URL is suitable for use as an external profile link.
///
/// This validates the URL's structure and permitted scheme. It does not
/// verify that the website exists or that the URL is safe to visit.
pub fn is_valid_url(url: &str) -> bool {
    // Set a reasonable URL length range.
    //
    // URLs can be considerably longer than names, so allow up to 2048
    // bytes. The length is measured in bytes, not characters.
    const MIN_URL_LENGTH: usize = 1;
    const MAX_URL_LENGTH: usize = 2048;

    if url.len() < MIN_URL_LENGTH || url.len() > MAX_URL_LENGTH {
        return false;
    }

    // Leading and trailing whitespace is not allowed.
    //
    // For example:
    // "https://example.com"  -> valid
    // " https://example.com" -> invalid
    // "https://example.com " -> invalid
    //
    // We reject rather than silently modifying the submitted URL.
    if url != url.trim() {
        return false;
    }

    // Whitespace is not allowed anywhere in the URL.
    //
    // For example:
    // "https://example.com/my page" -> invalid
    // "https://example.com/my%20page" -> valid
    //
    // Spaces inside URL components should be percent-encoded as %20.
    //
    // Control characters, such as newlines, tabs, and null characters,
    // are also rejected because they are not appropriate in this input.
    if url.chars().any(char::is_whitespace)
        || url.chars().any(char::is_control)
    {
        return false;
    }

    // Parse the URL instead of trying to validate its syntax manually.
    //
    // The URL parser checks the URL's structure and lets us inspect
    // individual components, such as its scheme and host.
    //
    // If parsing fails, the input is not an acceptable URL.
    let parsed = match url::Url::parse(url) {
        Ok(parsed) => parsed,
        Err(_) => return false,
    };

    // Only HTTP and HTTPS URLs are allowed.
    //
    // These are the schemes used for ordinary web links.
    // Other schemes, such as "javascript:", "data:", "file:", and
    // "mailto:", are not accepted as external website links.
    if !matches!(parsed.scheme(), "http" | "https") {
        return false;
    }

    // A host identifies the website's server.
    //
    // For example, the host in "https://github.com/example" is
    // "github.com".
    //
    // Require a host because this function validates website links,
    // not URLs referring to other kinds of resources.
    if parsed.host_str().is_none() {
        return false;
    }

    // Reject URLs containing embedded credentials.
    //
    // A URL can contain a username and password before the host:
    // "https://username:password@example.com"
    //
    // These credentials are unnecessary for ordinary profile links
    // and could mislead users about which website they are visiting.
    //
    // An empty username and no password are acceptable.
    if !parsed.username().is_empty() || parsed.password().is_some() {
        return false;
    }

    // The URL has passed our structural and scheme checks.
    //
    // This does not guarantee that the destination exists, is trustworthy,
    // or will remain safe. It only validates the URL's accepted format.
    //TODO check if there is more to reject here.
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---- is_valid_name ----
    #[test]
    fn rejects_angle_brackets() {
        assert!(!is_valid_name("<script>alert(1)</script>"));
        assert!(!is_valid_name("Alice <b>Smith</b>"));
        assert!(!is_valid_name("Ali<ce"));
        assert!(!is_valid_name("Alice>"));
    }
    #[test]
    fn accepts_a_normal_name() {
        assert!(is_valid_name("Alice"));
    }

    #[test]
    fn rejects_name_below_minimum_length() {
        assert!(!is_valid_name("A")); // 1 char, MIN is 2
    }

    #[test]
    fn accepts_name_at_minimum_length() {
        assert!(is_valid_name("Al")); // exactly 2 chars
    }

    #[test]
    fn rejects_name_above_maximum_length() {
        let name = "A".repeat(101); // MAX is 100
        assert!(!is_valid_name(&name));
    }

    #[test]
    fn accepts_name_at_maximum_length() {
        let name = "A".repeat(100);
        assert!(is_valid_name(&name));
    }

    #[test]
    fn rejects_leading_whitespace() {
        assert!(!is_valid_name(" Alice"));
    }

    #[test]
    fn rejects_trailing_whitespace() {
        assert!(!is_valid_name("Alice "));
    }

    #[test]
    fn rejects_control_characters() {
        assert!(!is_valid_name("Alice\n"));
        assert!(!is_valid_name("Ali\0ce"));
    }

    #[test]
    fn accepts_name_with_internal_space() {
        // e.g. "Alice Smith" -- internal whitespace is fine,
        // only leading/trailing is rejected.
        assert!(is_valid_name("Alice Smith"));
    }

    // ---- is_valid_email ----

    #[test]
    fn accepts_a_normal_email() {
        assert!(is_valid_email("user@example.com"));
    }

    #[test]
    fn rejects_missing_at_symbol() {
        assert!(!is_valid_email("userexample.com"));
    }

    #[test]
    fn rejects_multiple_at_symbols() {
        assert!(!is_valid_email("user@ex@ample.com"));
    }

    #[test]
    fn rejects_empty_local_part() {
        assert!(!is_valid_email("@example.com"));
    }

    #[test]
    fn rejects_empty_domain() {
        assert!(!is_valid_email("user@"));
    }

    #[test]
    fn rejects_domain_without_dot() {
        assert!(!is_valid_email("user@example"));
    }

    #[test]
    fn rejects_whitespace_anywhere() {
        assert!(!is_valid_email("user name@example.com"));
        assert!(!is_valid_email("user@ example.com"));
        assert!(!is_valid_email("user@example.com\n"));
    }

    #[test]
    fn rejects_local_part_starting_with_dot() {
        assert!(!is_valid_email(".user@example.com"));
    }

    #[test]
    fn rejects_local_part_ending_with_dot() {
        assert!(!is_valid_email("user.@example.com"));
    }

    #[test]
    fn rejects_consecutive_dots_in_local_part() {
        assert!(!is_valid_email("user..name@example.com"));
    }

    #[test]
    fn rejects_consecutive_dots_in_domain() {
        assert!(!is_valid_email("user@example..com"));
    }

    #[test]
    fn rejects_email_below_minimum_length() {
        assert!(!is_valid_email("a@")); // 2 chars, MIN is 3
    }

    #[test]
    fn rejects_email_above_maximum_length() {
        let long_local = "a".repeat(250);
        let email = format!("{long_local}@example.com"); // > 254 total
        assert!(!is_valid_email(&email));
    }

    // ---- is_valid_password ----

    #[test]
    fn accepts_password_at_minimum_length() {
        assert!(is_valid_password("12345678")); // exactly 8 chars
    }

    #[test]
    fn rejects_password_below_minimum_length() {
        assert!(!is_valid_password("1234567")); // 7 chars
    }

    #[test]
    fn accepts_password_at_maximum_length() {
        let password = "a".repeat(128);
        assert!(is_valid_password(&password));
    }

    #[test]
    fn rejects_password_above_maximum_length() {
        let password = "a".repeat(129);
        assert!(!is_valid_password(&password));
    }

    #[test]
    fn rejects_control_characters_in_password() {
        assert!(!is_valid_password("abcd\nefgh"));
        assert!(!is_valid_password("abcd\0efgh"));
    }

    #[test]
    fn accepts_password_with_special_characters() {
        assert!(is_valid_password("p@ssw0rd!#$"));
    }

    // ---- is_valid_url ----

    #[test]
    fn accepts_a_normal_https_url() {
        assert!(is_valid_url("https://example.com"));
    }

    #[test]
    fn accepts_a_normal_http_url() {
        assert!(is_valid_url("http://example.com"));
    }

    #[test]
    fn accepts_url_with_path() {
        assert!(is_valid_url("https://example.com/about"));
    }

    #[test]
    fn accepts_url_with_query_parameters() {
        assert!(is_valid_url(
            "https://example.com/about?source=profile"
        ));
    }

    #[test]
    fn accepts_url_with_fragment() {
        assert!(is_valid_url("https://example.com/about#team"));
    }

    #[test]
    fn accepts_url_with_percent_encoded_space() {
        assert!(is_valid_url("https://example.com/my%20page"));
    }

    #[test]
    fn rejects_empty_url() {
        assert!(!is_valid_url(""));
    }

    #[test]
    fn rejects_url_without_scheme() {
        assert!(!is_valid_url("example.com"));
    }

    #[test]
    fn rejects_url_without_host() {
        assert!(!is_valid_url("https://"));
    }

    #[test]
    fn rejects_malformed_url() {
        assert!(!is_valid_url("not a url"));
    }

    #[test]
    fn rejects_leading_whitespace_url() {
        assert!(!is_valid_url(" https://example.com"));
    }

    #[test]
    fn rejects_trailing_whitespace_url() {
        assert!(!is_valid_url("https://example.com "));
    }

    #[test]
    fn rejects_whitespace_inside_url() {
        assert!(!is_valid_url("https://example.com/my page"));
    }

    #[test]
    fn rejects_newline_inside_url() {
        assert!(!is_valid_url("https://example.com/\nabout"));
    }

    #[test]
    fn rejects_null_character_inside_url() {
        assert!(!is_valid_url("https://example.com/\0about"));
    }

    #[test]
    fn rejects_javascript_scheme() {
        assert!(!is_valid_url("javascript:alert(1)"));
    }

    #[test]
    fn rejects_data_scheme() {
        assert!(!is_valid_url("data:text/html,something"));
    }

    #[test]
    fn rejects_file_scheme() {
        assert!(!is_valid_url("file:///etc/passwd"));
    }

    #[test]
    fn rejects_mailto_scheme() {
        assert!(!is_valid_url("mailto:hello@example.com"));
    }

    #[test]
    fn rejects_url_with_embedded_username() {
        assert!(!is_valid_url("https://username@example.com"));
    }

    #[test]
    fn rejects_url_with_embedded_username_and_password() {
        assert!(!is_valid_url(
            "https://username:password@example.com"
        ));
    }

    #[test]
    fn rejects_url_above_maximum_length() {
        // The scheme and host are valid, but the complete URL exceeds
        // the maximum allowed length of 2048 bytes.
        let url = format!("https://example.com/{}", "a".repeat(2048));

        assert!(!is_valid_url(&url));
    }

    #[test]
    fn accepts_url_at_maximum_length() {
        // Construct a valid URL whose total length is exactly 2048 bytes.
        let prefix = "https://example.com/";
        let remaining_length = 2048 - prefix.len();
        let url = format!("{prefix}{}", "a".repeat(remaining_length));

        assert_eq!(url.len(), 2048);
        assert!(is_valid_url(&url));
    }
}