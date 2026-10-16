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
    //TODO think if this should be displayed bigger and in color? hard to see?
    statusElement.textContent = message;

    clearTimeout(statusTimeout);

    statusTimeout = setTimeout(() => {
        statusElement.textContent = "";
    }, 6000);
}