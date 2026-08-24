// signup/signup.js

const ERROR_MESSAGES = {
    "invalid-name": "Please enter a valid organization name.",
    "invalid-email": "Please enter a valid email address.",
    "invalid-password": "Password must be at least 8 characters long.",
    "passwords-mismatch": "Passwords do not match.",
    "email-taken": "An account with that email already exists.",
    "organization-exists": "An organization with that name already exists.",
    "server-error": "Something went wrong. Please try again.",
};

function showError(message) {
    const banner = document.getElementById("form-error");
    banner.textContent = message;
    banner.hidden = false;
    banner.scrollIntoView({ behavior: "smooth", block: "center" });
}

function hideError() {
    const banner = document.getElementById("form-error");
    banner.hidden = true;
}

// Show a banner if the server redirected back with an error.
function showServerErrorIfPresent() {
    const params = new URLSearchParams(window.location.search);
    const code = params.get("error");
    if (code && ERROR_MESSAGES[code]) {
        showError(ERROR_MESSAGES[code]);
    }
}

// Mirrors the server's validation rules so obvious mistakes are caught
// before a round trip. The server remains the source of truth.
function validateClientSide(form) {
    const name = form.name.value;
    const email = form.email.value;
    const password = form.password.value;
    const confirm = form.confirm.value;

    if (name.trim().length !== name.length || name.length < 2 || name.length > 100) {
        return "Please enter a valid organization name.";
    }

    if (!email.includes("@") || !email.split("@")[1]?.includes(".")) {
        return "Please enter a valid email address.";
    }

    if (password.length < 8 || password.length > 128) {
        return "Password must be at least 8 characters long.";
    }

    if (password !== confirm) {
        return "Passwords do not match.";
    }

    return null;
}

document.addEventListener("DOMContentLoaded", () => {
    showServerErrorIfPresent();

    const form = document.getElementById("signup-form");
    form.addEventListener("submit", (event) => {
        const error = validateClientSide(form);
        if (error) {
            event.preventDefault();
            showError(error);
        } else {
            hideError();
        }
    });
});