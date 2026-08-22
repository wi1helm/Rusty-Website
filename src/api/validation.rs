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
