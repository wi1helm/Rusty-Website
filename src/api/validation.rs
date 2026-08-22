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

#[cfg(test)]
mod tests {
    use super::*;

    // ---- is_valid_name ----

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
}