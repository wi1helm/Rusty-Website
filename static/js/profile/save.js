/* =========================================================
   profile/save.js  (EDITOR only)
========================================================= */

import { $ } from "../shared/helpers.js";
import { showStatus } from "./status.js";


async function postJson(url, body = undefined) {
    const options = {
        method: "POST",
        credentials: "same-origin",
        headers: {
            "Accept": "text/html, application/json"
        }
    };

    if (body !== undefined) {
        options.headers["Content-Type"] = "application/json";
        options.body = JSON.stringify(body);
    }

    return fetch(url, options);
}


async function saveProfileData(state) {
    return postJson("/api/profile/update", {
        display_name: state.displayName,
        about: state.about,
        links: state.links.map(link => ({
            label: link.label,
            url: link.url
        }))
    });
}


async function saveOrganizationGraph() {
    // TODO: Replace with actual graph state once implemented.
    return postJson("/api/profile/update-graph", {});
}


export function initSave(state) {
    const saveButton = $("#save-profile");

    if (!saveButton) return;

    saveButton.addEventListener("click", async () => {
        const originalText = saveButton.textContent;

        saveButton.disabled = true;
        saveButton.textContent = "Saving…";

        try {
            // Save profile information.
            const profileResponse = await saveProfileData(state);

            if (
                profileResponse.redirected &&
                new URL(profileResponse.url).searchParams.has("error")
            ) {
                window.location.assign(profileResponse.url);
                return;
            }

            if (!profileResponse.ok) {
                throw new Error(`Profile save failed with status ${profileResponse.status}.`);
            }

            // Save organization graph.
            const graphResponse = await saveOrganizationGraph();

            if (!graphResponse.ok) {
                throw new Error(`Graph save failed with status ${graphResponse.status}.`);
            }

            showStatus("Profile and organization graph saved. Reloading…");
            window.location.reload();

        } catch (error) {
            console.error("Save failed:", error);
            showStatus("Could not save everything. Please try again.");

        } finally {
            saveButton.disabled = false;
            saveButton.textContent = originalText;
        }
    });
}