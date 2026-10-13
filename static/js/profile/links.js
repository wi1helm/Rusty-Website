/* =========================================================
   profile/links.js  (EDITOR only)
   Link bar: render, add, edit, remove.
   Icons come from shared/link-icons.js (same as public page).
========================================================= */

import { $, $$, createElement, createSvgIcon, icons } from "../shared/helpers.js";
import { renderLinkIcon } from "../shared/link-icons.js";
import { makeButton } from "./buttons.js";
import { openDialog, closeDialog } from "./dialogs.js";
import { showStatus } from "./status.js";


function createLinkId() {
    if (typeof crypto !== "undefined" && typeof crypto.randomUUID === "function") {
        return crypto.randomUUID();
    }

    return Date.now().toString(36) + Math.random().toString(36).slice(2);
}


function normalizeLinkUrl(value) {
    const raw = value.trim();

    if (!raw) {
        throw new Error("Please enter a URL.");
    }

    let parsed;

    try {
        parsed = new URL(raw);
    } catch {
        throw new Error("Please enter a valid URL, including https://.");
    }

    if (parsed.protocol !== "https:" && parsed.protocol !== "http:") {
        throw new Error("Only HTTP and HTTPS links are supported.");
    }

    if (!parsed.hostname) {
        throw new Error("Please enter a valid website address.");
    }

    return parsed.href;
}


function readInitialLinks() {
    const container = $("#initial-profile-links");

    if (!container) {
        return [];
    }

    return $$(".initial-profile-link", container)
        .map(element => ({
            id: createLinkId(),
            label: element.dataset.label || "",
            url: element.dataset.url || ""
        }))
        .filter(link => link.label && link.url);
}


export function initLinks(state) {
    const linkBar = $("#profile-link-bar");

    const dialog = $("#link-dialog");
    const form = $("#link-form");
    const dialogTitle = $("#link-dialog-title");
    const labelInput = $("#link-label-input");
    const urlInput = $("#link-url-input");
    const formError = $("#link-form-error");
    const formSubmit = $("#link-form-submit");

    const profileSlug = linkBar ? linkBar.dataset.profileSlug : "";

    let editingLinkId = null;

    state.links = readInitialLinks();


    /* ---------- Rendering ---------- */

    // "View my public page" shortcut. Editor only, not stored in state.links.
    function renderPublicProfileLink() {
        const wrapper = createElement("div", "profile-link-item profile-public-link-item");
        const anchor = createElement("a", "profile-link-anchor");

        anchor.href = `/${profileSlug}`;

        const iconContainer = createElement("span", "profile-link-icon");
        iconContainer.appendChild(createSvgIcon(icons.globe, 19));

        anchor.append(
            iconContainer,
            createElement("span", "profile-link-label", "Public profile")
        );

        wrapper.appendChild(anchor);

        return wrapper;
    }

    function renderExternalLink(link) {
        const wrapper = createElement("div", "profile-link-item");
        const anchor = createElement("a", "profile-link-anchor");

        anchor.href = link.url;
        anchor.target = "_blank";
        anchor.rel = "noopener noreferrer";

        const iconContainer = createElement("span");
        renderLinkIcon(iconContainer, link.url);

        anchor.append(
            iconContainer,
            createElement("span", "profile-link-label", link.label)
        );

        const actions = createElement("div", "profile-link-actions");

        const editButton = makeButton(`Edit ${link.label}`, "edit", "profile-link-action");
        editButton.addEventListener("click", () => openLinkEditor(link.id));

        const removeButton = makeButton(`Remove ${link.label}`, "trash", "profile-link-action");
        removeButton.addEventListener("click", () => {
            if (!window.confirm(`Remove "${link.label}" from your profile?`)) {
                return;
            }

            state.links = state.links.filter(item => item.id !== link.id);
            renderLinks();

            showStatus("Link removed from the editor. Save changes to persist it.");
        });

        actions.append(editButton, removeButton);
        wrapper.append(anchor, actions);

        return wrapper;
    }

    function renderAddLinkButton() {
        const button = makeButton("Add a link", "plus", "profile-add-link");
        button.addEventListener("click", () => openLinkEditor(null));
        return button;
    }

    function renderLinks() {
        if (!linkBar) {
            return;
        }

        linkBar.replaceChildren();
        linkBar.appendChild(renderPublicProfileLink());

        for (const link of state.links) {
            linkBar.appendChild(renderExternalLink(link));
        }

        linkBar.appendChild(renderAddLinkButton());
    }


    /* ---------- Dialog ---------- */

    function showLinkError(message) {
        if (formError) {
            formError.textContent = message;
            formError.hidden = false;
        }
    }

    function clearLinkError() {
        if (formError) {
            formError.textContent = "";
            formError.hidden = true;
        }
    }

    function openLinkEditor(linkId) {
        if (!dialog || !form || !labelInput || !urlInput) {
            return;
        }

        clearLinkError();
        editingLinkId = linkId;

        if (linkId) {
            const link = state.links.find(item => item.id === linkId);

            if (!link) {
                return;
            }

            dialogTitle.textContent = "Edit link";
            formSubmit.textContent = "Apply changes";
            labelInput.value = link.label;
            urlInput.value = link.url;
        } else {
            dialogTitle.textContent = "Add a link";
            formSubmit.textContent = "Add link";
            labelInput.value = "";
            urlInput.value = "";
        }

        openDialog(dialog);

        requestAnimationFrame(() => labelInput.focus());
    }

    if (form) {
        form.addEventListener("submit", event => {
            event.preventDefault();
            clearLinkError();

            const label = labelInput.value.trim();

            if (!label) {
                showLinkError("Please enter a label for this link.");
                labelInput.focus();
                return;
            }

            let url;

            try {
                url = normalizeLinkUrl(urlInput.value);
            } catch (error) {
                showLinkError(error.message);
                urlInput.focus();
                return;
            }

            if (editingLinkId) {
                const existing = state.links.find(link => link.id === editingLinkId);

                if (existing) {
                    existing.label = label;
                    existing.url = url;
                }
            } else {
                state.links.push({ id: createLinkId(), label, url });
            }

            renderLinks();
            closeDialog(dialog);

            showStatus("Links updated in the editor. Save changes to persist them.");
        });
    }

    renderLinks();
}