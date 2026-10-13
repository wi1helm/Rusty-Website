/* =========================================================
   profile/status.js  (EDITOR only)
========================================================= */

import { $ } from "../shared/helpers.js";

let statusTimeout = null;

export function showStatus(message) {
    const statusElement = $("#profile-editor-status");

    if (!statusElement) {
        return;
    }

    statusElement.textContent = message;

    clearTimeout(statusTimeout);

    statusTimeout = setTimeout(() => {
        statusElement.textContent = "";
    }, 6000);
}