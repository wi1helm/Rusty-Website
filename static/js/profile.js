document.addEventListener("DOMContentLoaded", () => {
    setupInlineEditors();
    setupImageUploads();
});


/*
 * --------------------------------------------------------------------------
 * Inline editors
 * --------------------------------------------------------------------------
 *
 * Elements:
 *
 *   [data-edit="about"]
 *   [data-editor="about"]
 *   [data-content="about"]
 *
 * Clicking the edit button hides the normal content and shows the editor.
 */

function setupInlineEditors() {
    const editButtons = document.querySelectorAll("[data-edit]");

    editButtons.forEach((button) => {
        button.addEventListener("click", () => {
            const field = button.dataset.edit;

            if (!field) {
                return;
            }

            openEditor(field);
        });
    });

    document.querySelectorAll(".inline-editor").forEach((editor) => {
        const field = editor.dataset.editor;

        if (!field) {
            return;
        }

        const buttons = editor.querySelectorAll("button");

        if (buttons.length >= 2) {
            const saveButton = buttons[0];
            const cancelButton = buttons[1];

            saveButton.addEventListener("click", () => {
                saveEditor(field);
            });

            cancelButton.addEventListener("click", () => {
                cancelEditor(field);
            });
        }
    });
}


function openEditor(field) {
    const editor = document.querySelector(
        `[data-editor="${field}"]`
    );

    if (!editor) {
        return;
    }

    const content = document.querySelector(
        `[data-content="${field}"]`
    );

    if (content) {
        content.hidden = true;
    }

    editor.hidden = false;

    const input = editor.querySelector(
        "input, textarea"
    );

    if (input) {
        input.focus();

        // Put cursor at the end rather than selecting the whole value.
        if (typeof input.setSelectionRange === "function") {
            const length = input.value.length;
            input.setSelectionRange(length, length);
        }
    }
}


function cancelEditor(field) {
    const editor = document.querySelector(
        `[data-editor="${field}"]`
    );

    if (!editor) {
        return;
    }

    const content = document.querySelector(
        `[data-content="${field}"]`
    );

    editor.hidden = true;

    if (content) {
        content.hidden = false;
    }
}


/*
 * --------------------------------------------------------------------------
 * Save inline editor
 * --------------------------------------------------------------------------
 */

async function saveEditor(field) {
    const editor = document.querySelector(
        `[data-editor="${field}"]`
    );

    if (!editor) {
        return;
    }

    const input = editor.querySelector(
        "input, textarea"
    );

    if (!input) {
        return;
    }

    const value = input.value.trim();

    if (!value && input.required) {
        input.focus();
        return;
    }

    const saveButton = editor.querySelector(
        "button"
    );

    setEditorBusy(editor, true);

    try {
        const response = await fetch("/api/profile/edit", {
            method: "POST",
            headers: {
                "Content-Type": "application/json",
            },
            credentials: "same-origin",
            body: JSON.stringify({
                [field]: value,
            }),
        });

        if (!response.ok) {
            throw new Error(
                `Profile update failed: ${response.status}`
            );
        }

        updateProfileField(field, value);

        editor.hidden = true;

        const content = document.querySelector(
            `[data-content="${field}"]`
        );

        if (content) {
            content.hidden = false;
        }
    } catch (error) {
        console.error(error);

        /*
         * For now we keep the error handling deliberately simple.
         *
         * Later we can add an inline error message to the editor instead
         * of using alert().
         */
        console.error("Unable to save profile changes.");
    } finally {
        setEditorBusy(editor, false);
    }
}


/*
 * Update the visible profile after a successful save.
 */

function updateProfileField(field, value) {
    if (field === "display_name") {
        const heading = document.querySelector(
            ".profile-display-name-row h1"
        );

        if (heading) {
            heading.textContent = value;
        }

        return;
    }

    const content = document.querySelector(
        `[data-content="${field}"]`
    );

    if (!content) {
        return;
    }

    if (!value) {
        content.innerHTML = `
            <p class="profile-empty">
                No description yet.
            </p>
        `;

        return;
    }

    if (field === "about") {
        content.innerHTML = "";

        const paragraph = document.createElement("p");
        paragraph.className = "profile-about";
        paragraph.textContent = value;

        content.appendChild(paragraph);
    }
}


/*
 * Disable editor buttons while a request is in progress.
 */

function setEditorBusy(editor, busy) {
    const buttons = editor.querySelectorAll("button");

    buttons.forEach((button) => {
        button.disabled = busy;
    });
}


/*
 * --------------------------------------------------------------------------
 * Image uploads
 * --------------------------------------------------------------------------
 *
 * Clicking either:
 *
 *   [data-image-type="logo"]
 *   [data-image-type="banner"]
 *
 * opens a native file picker.
 *
 * The actual file input is created dynamically so we don't need to put
 * <input type="file"> elements into the visible profile markup.
 */

function setupImageUploads() {
    const imageButtons = document.querySelectorAll(
        "[data-image-type]"
    );

    imageButtons.forEach((button) => {
        button.addEventListener("click", () => {
            const type = button.dataset.imageType;

            if (!type) {
                return;
            }

            chooseImage(type);
        });
    });
}


function chooseImage(type) {
    const input = document.createElement("input");

    input.type = "file";
    input.accept = "image/*";

    input.addEventListener("change", async () => {
        const file = input.files?.[0];

        if (!file) {
            return;
        }

        await uploadImage(type, file);
    });

    input.click();
}


/*
 * Upload the selected image to Axum.
 */

async function uploadImage(type, file) {
    const button = document.querySelector(
        `[data-image-type="${type}"]`
    );

    if (button) {
        button.disabled = true;
    }

    try {
        const formData = new FormData();

        formData.append("image", file);
        formData.append("type", type);

        const response = await fetch(
            "/api/profile/image",
            {
                method: "POST",
                body: formData,
                credentials: "same-origin",
            }
        );

        if (!response.ok) {
            throw new Error(
                `Image upload failed: ${response.status}`
            );
        }

        const result = await response.json();

        /*
         * Expected response:
         *
         * {
         *     "url": "http://localhost:9000/my-app/...",
         *     "alt": "W Jerrestål AB logo"
         * }
         *
         * We can change this later if the API response is different.
         */

        if (!result.url) {
            throw new Error(
                "Image upload response did not contain a URL"
            );
        }

        updateProfileImage(type, result);
    } catch (error) {
        console.error(error);
        console.error("Unable to upload image.");
    } finally {
        if (button) {
            button.disabled = false;
        }
    }
}


/*
 * Update the displayed image without reloading the page.
 */

function updateProfileImage(type, image) {
    if (type === "logo") {
        updateLogo(image);
        return;
    }

    if (type === "banner") {
        updateBanner(image);
    }
}


function updateLogo(image) {
    const button = document.querySelector(
        '[data-image-type="logo"]'
    );

    if (!button) {
        return;
    }

    let img = button.querySelector("img");

    if (!img) {
        img = document.createElement("img");
        img.className = "profile-logo";

        button.replaceChildren(img);
    }

    img.src = image.url;
    img.alt = image.alt ?? "";
}


function updateBanner(image) {
    const wrapper = document.querySelector(
        ".profile-banner-wrapper"
    );

    if (!wrapper) {
        return;
    }

    let img = wrapper.querySelector(".profile-banner");

    if (!img || img.tagName !== "IMG") {
        img = document.createElement("img");
        img.className = "profile-banner";

        const editButton = wrapper.querySelector(
            ".profile-banner-edit"
        );

        if (editButton) {
            wrapper.insertBefore(img, editButton);
        } else {
            wrapper.appendChild(img);
        }
    }

    img.src = image.url;
    img.alt = image.alt ?? "";
}