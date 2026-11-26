/* =========================================================
   profile/save.js  (EDITOR only)
========================================================= */

import { $ } from "../shared/helpers.js";
import { getCurrentStatus } from "../shared/graph/graph.js";
import { apiFetch } from "../shared/api.js";
import { showStatus } from "./status.js";


async function postJson(url, body = undefined) {
    const options = {
        method: "POST",
        headers: {}
    };

    if (body !== undefined) {
        options.headers["Content-Type"] = "application/json";
        options.body = JSON.stringify(body);
    }

    const { response } = await apiFetch(url, options);
    return response;
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


async function saveOrganizationGraph(state) {
    return postJson("/api/profile/update-graph", {
        positions: state.graph.positions.map(position => ({
            id: position.id,
            client_id: position.client_id,
            title: position.title,
            description: position.description,
            current_status: getCurrentStatus(position)?.status,
            statuses: position.statuses.map(status => ({
                status: status.status,
                information: status.information
            }))
        })),
        connections: state.graph.connections.map(connection => ({
            from: connection.from,
            to: connection.to
        }))
    });
}


export function initSave(state) {
    const saveButton = $("#save-profile");

    if (!saveButton) return;

    saveButton.addEventListener("click", async () => {
        const originalText = saveButton.textContent;
        let profileSaved = false;

        saveButton.disabled = true;
        saveButton.textContent = "Saving…";

        try {
            // Save profile information.
            const profileResponse = await saveProfileData(state);

            if (!profileResponse.ok) {
                throw new Error(`Profile save failed with status ${profileResponse.status}.`);
            }
            profileSaved = true;

            // Save organization graph.
            const graphResponse = await saveOrganizationGraph(state);

            if (!graphResponse.ok) {
                throw new Error(`Graph save failed with status ${graphResponse.status}.`);
            }

            showStatus("Profile and organization graph saved. Reloading…");
            window.location.reload();

        } catch (error) {
            console.error("Save failed:", error);
            showStatus(profileSaved
                ? `Profile details were saved, but the organization graph could not be saved. ${error.message}`
                : error.message || "Could not save your profile. Please try again.");

        } finally {
            saveButton.disabled = false;
            saveButton.textContent = originalText;
        }
    });
}