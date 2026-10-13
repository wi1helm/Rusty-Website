/* =========================================================
   profile/dialogs.js  (EDITOR only)
   Helpers for native <dialog> elements.
========================================================= */

import { $$ } from "../shared/helpers.js";

export function openDialog(dialog) {
    if (!dialog) {
        return;
    }

    if (typeof dialog.showModal === "function") {
        if (!dialog.open) {
            dialog.showModal();
        }
    } else {
        dialog.setAttribute("open", "");
    }
}

export function closeDialog(dialog) {
    if (!dialog) {
        return;
    }

    if (typeof dialog.close === "function" && dialog.open) {
        dialog.close();
    } else {
        dialog.removeAttribute("open");
    }
}

export function initDialogs() {
    $$("[data-close-dialog]").forEach(button => {
        button.addEventListener("click", () => {
            closeDialog(document.getElementById(button.dataset.closeDialog));
        });
    });

    // Clicking the backdrop closes the dialog.
    $$(".profile-dialog").forEach(dialog => {
        dialog.addEventListener("click", event => {
            if (event.target === dialog) {
                closeDialog(dialog);
            }
        });
    });
}