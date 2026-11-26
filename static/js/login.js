import { apiPostJson } from "./shared/api.js";

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


document.addEventListener("DOMContentLoaded", () => {
    const form = document.getElementById("login-form");
    form.addEventListener("submit", async (event) => {
        event.preventDefault();
        hideError();
        const submitButton = form.querySelector("button[type=submit]");
        submitButton.disabled = true;

        try {
            const { data } = await apiPostJson(form.action, Object.fromEntries(new FormData(form)));

            if (data?.redirectTo) {
                window.location.assign(data.redirectTo);
            } else {
                showError("Your request was completed, but the next page could not be opened.");
            }
        } catch (error) {
            showError(error.message);
        } finally {
            submitButton.disabled = false;
        }
    });
});