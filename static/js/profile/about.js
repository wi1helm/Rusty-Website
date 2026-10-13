/* =========================================================
   profile/about.js  (EDITOR only)
========================================================= */

import { $ } from "../shared/helpers.js";
import { openDialog, closeDialog } from "./dialogs.js";
import { showStatus } from "./status.js";

export function initAbout(state) {
    const content = $("#profile-about-content");
    const editButton = $("#edit-about");
    const dialog = $("#about-dialog");
    const form = $("#about-form");
    const input = $("#about-input");
    const characterCount = $("#about-character-count");

    state.about = content ? content.textContent.trim() : "";

    function render() {
        if (!content) {
            return;
        }

        content.textContent = state.about;
        content.classList.toggle("is-empty", state.about.length === 0);
    }

    function updateCharacterCount() {
        if (characterCount && input) {
            characterCount.textContent = String(input.value.length);
        }
    }

    render();

    if (!content || !editButton || !dialog || !form || !input) {
        return;
    }

    editButton.addEventListener("click", () => {
        input.value = state.about;
        updateCharacterCount();
        openDialog(dialog);

        requestAnimationFrame(() => input.focus());
    });

    input.addEventListener("input", updateCharacterCount);

    form.addEventListener("submit", event => {
        event.preventDefault();

        state.about = input.value;
        render();
        closeDialog(dialog);

        showStatus("About updated in the editor. Save changes to persist it.");
    });
}