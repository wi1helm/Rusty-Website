/* =========================================================
   profile/save-publish.js  (EDITOR only)
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


export function initSavePublish(state) {
    const saveButton = $("#save-profile");
    const publishButton = $("#publish-profile");

    if (saveButton) {
        saveButton.addEventListener("click", async () => {
            const originalText = saveButton.textContent;
            saveButton.disabled = true;
            saveButton.textContent = "Saving…";

            try {
                const response = await postJson("/api/profile/edit", {
                    display_name: state.displayName,
                    about: state.about,
                    links: state.links.map(link => ({
                        label: link.label,
                        url: link.url
                    }))
                });

                // The Rust handler currently redirects on both success and
                // validation errors. A validation redirect includes ?error=.
                if (response.redirected && new URL(response.url).searchParams.has("error")) {
                    window.location.assign(response.url);
                    return;
                }

                if (!response.ok) {
                    throw new Error(`Save failed with status ${response.status}.`);
                }

                showStatus("Profile saved. Reloading…");
                window.location.reload();
            } catch (error) {
                console.error("Profile save failed:", error);
                showStatus("Could not save your profile. Please try again.");
            } finally {
                saveButton.disabled = false;
                saveButton.textContent = originalText;
            }
        });
    }

    if (publishButton) {
        publishButton.addEventListener("click", async () => {
            if (!window.confirm("Publish the current saved profile version? This makes it public.")) {
                return;
            }

            const originalText = publishButton.textContent;
            publishButton.disabled = true;
            publishButton.textContent = "Publishing…";

            try {
                const response = await postJson("/api/profile/publish");

                if (!response.ok) {
                    throw new Error(`Publish failed with status ${response.status}.`);
                }

                showStatus("Profile published. Reloading…");
                window.location.reload();
            } catch (error) {
                console.error("Profile publish failed:", error);
                showStatus("Could not publish your profile. Please try again.");
            } finally {
                publishButton.disabled = false;
                publishButton.textContent = originalText;
            }
        });
    }
}