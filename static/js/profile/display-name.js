/* =========================================================
   profile/display-name.js  (EDITOR only)
========================================================= */

import { $ } from "../shared/helpers.js";
import { openDialog, closeDialog } from "./dialogs.js";
import { showStatus } from "./status.js";

export function initDisplayName(state) {
    const nameElement = $("#profile-display-name");
    const editButton = $("#edit-display-name");
    const dialog = $("#display-name-dialog");
    const form = $("#display-name-form");
    const input = $("#display-name-input");

    state.displayName = nameElement ? nameElement.textContent.trim() : "";

    if (!nameElement || !editButton || !dialog || !form || !input) {
        return;
    }

    editButton.addEventListener("click", () => {
        input.value = state.displayName;
        openDialog(dialog);

        requestAnimationFrame(() => {
            input.focus();
            input.select();
        });
    });

    form.addEventListener("submit", event => {
        event.preventDefault();

        const nextName = input.value.trim();

        if (!nextName) {
            input.setCustomValidity("Please enter an organization name.");
            input.reportValidity();
            return;
        }

        input.setCustomValidity("");

        state.displayName = nextName;
        nameElement.textContent = nextName;
        document.title = `${nextName} - Profile`;

        closeDialog(dialog);

        showStatus("Display name updated in the editor. Save changes to persist it.");
    });

    input.addEventListener("input", () => {
        input.setCustomValidity("");
    });
}