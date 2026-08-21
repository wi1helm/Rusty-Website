// routes/api/validation.rs

pub enum ValidationError {
    InvalidEmail,
    InvalidPassword,
    InvalidName,
    PasswordMismatch,
}

pub fn is_valid_name(name: &str) -> bool {
    const MIN_NAME_LENGTH: usize = 2;
    const MAX_NAME_LENGTH: usize = 100;

    if name.len() < MIN_NAME_LENGTH || name.len() > MAX_NAME_LENGTH {
        return false;
    }

    // Don't allow leading/trailing whitespace.
    if name != name.trim() {
        return false;
    }

    // Don't allow control characters such as \n, \r, \0, etc.
    if name.chars().any(char::is_control) {
        return false;
    }

    true
}

pub fn is_valid_email(email: &str) -> bool {
    // The maximum length of an email address.
    // 254 characters is a commonly used practical limit.
    const MAX_EMAIL_LENGTH: usize = 254;

    // For our purposes, the shortest valid-looking email is "a@b".
    const MIN_EMAIL_LENGTH: usize = 3;

    // Email addresses must be within our allowed length range.
    if email.len() > MAX_EMAIL_LENGTH || email.len() < MIN_EMAIL_LENGTH {
        return false;
    }

    // An email address must contain exactly one '@'.
    if email.matches('@').count() != 1 {
        return false;
    }

    // Split the email into:
    // - local: the part before '@'
    // - domain: the part after '@'
    //
    // We know there is exactly one '@', so split_once() should succeed.
    let Some((local, domain)) = email.split_once('@') else {
        return false;
    };

    // Both parts must contain something.
    //
    // Examples:
    // "@example.com" -> invalid
    // "user@"        -> invalid
    if local.is_empty() || domain.is_empty() {
        return false;
    }

    // Email addresses should not contain whitespace.
    // This catches spaces, tabs, newlines, etc.
    if email.chars().any(char::is_whitespace) {
        return false;
    }

    // For now, require the domain to contain a dot.
    //
    // "user@example.com" -> valid
    // "user@example"     -> invalid
    if !domain.contains('.') {
        return false;
    }

    // The local part should not start or end with a dot.
    //
    // ".user@example.com" -> invalid
    // "user.@example.com" -> invalid
    if local.starts_with('.') || local.ends_with('.') {
        return false;
    }

    // Don't allow consecutive dots.
    //
    // "user..name@example.com" -> invalid
    // "user@example..com"      -> invalid
    if local.contains("..") || domain.contains("..") {
        return false;
    }
    // Emil is valid but is stil not trusted to be an actual email. Just passes these tests.
    true
}

pub fn is_valid_password(password: &str) -> bool {
    const MIN_PASSWORD_LENGTH: usize = 8;
    const MAX_PASSWORD_LENGTH: usize = 128;

    // Password must be within our allowed length range.
    if password.len() < MIN_PASSWORD_LENGTH || password.len() > MAX_PASSWORD_LENGTH {
        return false;
    }

    // Don't allow control characters such as \n, \r, \0, etc.
    if password.chars().any(char::is_control) {
        return false;
    }

    true
}

