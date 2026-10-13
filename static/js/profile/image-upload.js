/* =========================================================
   profile/image-upload.js  (EDITOR only)
   Banner and logo upload modal.
========================================================= */

import { $, $$ } from "../shared/helpers.js";


const ALLOWED_TYPES = ["image/jpeg", "image/png", "image/webp"];


export function initImageUpload() {
    const modal = $("#image-upload-modal");
    const input = $("#image-upload-input");
    const preview = $("#image-upload-preview");
    const previewImage = preview ? $("img", preview) : null;
    const title = $("#image-upload-title");
    const description = $("#image-upload-description");
    const closeButton = $(".image-upload-close");
    const cancelButton = $("#image-upload-cancel");
    const submitButton = $("#image-upload-submit");
    const dropzone = $(".image-upload-dropzone");

    let currentImageType = null;
    let currentUploadUrl = null;
    let selectedFile = null;
    let previewObjectUrl = null;
    let previouslyFocused = null;


    function revokePreviewUrl() {
        if (previewObjectUrl) {
            URL.revokeObjectURL(previewObjectUrl);
            previewObjectUrl = null;
        }
    }

    function clearPreview() {
        revokePreviewUrl();

        if (previewImage) {
            previewImage.removeAttribute("src");
        }

        if (preview) {
            preview.hidden = true;
        }
    }

    function closeModal() {
        if (modal) {
            modal.hidden = true;
        }

        selectedFile = null;
        currentImageType = null;
        currentUploadUrl = null;

        if (input) {
            input.value = "";
        }

        if (submitButton) {
            submitButton.disabled = false;
        }

        clearPreview();

        if (previouslyFocused) {
            previouslyFocused.focus();
            previouslyFocused = null;
        }
    }

    function openModal(editor) {
        if (!modal || !input || !preview || !previewImage) {
            console.error("Profile image upload UI is missing.");
            return;
        }

        previouslyFocused = editor;
        currentImageType = editor.dataset.imageType;
        currentUploadUrl = editor.dataset.uploadUrl;
        selectedFile = null;

        input.value = "";
        clearPreview();

        if (currentImageType === "logo") {
            title.textContent = "Upload logo";
            description.textContent =
                "Choose a square image. Recommended size: 400 × 400 pixels.";
        } else if (currentImageType === "banner") {
            title.textContent = "Upload banner";
            description.textContent =
                "Choose a banner image. Recommended size: 1500 × 500 pixels.";
        } else {
            return;
        }

        modal.hidden = false;

        if (closeButton) {
            closeButton.focus();
        }
    }

    function selectFile(file) {
        if (!file) {
            return;
        }

        if (!ALLOWED_TYPES.includes(file.type)) {
            window.alert("Please choose a JPEG, PNG, or WebP image.");
            return;
        }

        selectedFile = file;
        clearPreview();

        previewObjectUrl = URL.createObjectURL(file);
        previewImage.src = previewObjectUrl;
        preview.hidden = false;
    }


    /* ---------- Open triggers ---------- */

    $$(".profile-image-editor").forEach(editor => {
        editor.addEventListener("click", () => openModal(editor));

        editor.addEventListener("keydown", event => {
            if (event.key === "Enter" || event.key === " ") {
                event.preventDefault();
                openModal(editor);
            }
        });
    });


    /* ---------- Close triggers ---------- */

    if (closeButton) closeButton.addEventListener("click", closeModal);
    if (cancelButton) cancelButton.addEventListener("click", closeModal);

    if (modal) {
        modal.addEventListener("click", event => {
            if (event.target === modal) {
                closeModal();
            }
        });
    }

    // Native <dialog>s handle Escape themselves; this overlay is custom.
    document.addEventListener("keydown", event => {
        if (event.key === "Escape" && modal && !modal.hidden) {
            event.preventDefault();
            closeModal();
        }
    });


    /* ---------- File selection ---------- */

    if (input) {
        input.addEventListener("change", () => {
            const file = input.files?.[0];

            if (file) {
                selectFile(file);
            }
        });
    }

    if (dropzone) {
        dropzone.addEventListener("dragover", event => {
            event.preventDefault();
            dropzone.classList.add("dragging");
        });

        dropzone.addEventListener("dragleave", event => {
            if (event.relatedTarget && dropzone.contains(event.relatedTarget)) {
                return;
            }

            dropzone.classList.remove("dragging");
        });

        dropzone.addEventListener("drop", event => {
            event.preventDefault();
            dropzone.classList.remove("dragging");

            const file = event.dataTransfer?.files?.[0];

            if (file) {
                selectFile(file);
            }
        });
    }


    /* ---------- Upload ---------- */

    if (submitButton) {
        submitButton.addEventListener("click", async () => {
            if (!selectedFile) {
                window.alert("Please choose an image first.");
                return;
            }

            if (!currentUploadUrl || !currentImageType) {
                window.alert("The image upload configuration is missing.");
                return;
            }

            submitButton.disabled = true;

            try {
                const formData = new FormData();
                formData.append("image", selectedFile);
                formData.append("image_type", currentImageType);

                const response = await fetch(currentUploadUrl, {
                    method: "POST",
                    body: formData
                });

                if (!response.ok) {
                    throw new Error(`Image upload failed with status ${response.status}.`);
                }

                closeModal();
                window.location.reload();
            } catch (error) {
                console.error("Profile image upload failed:", error);
                window.alert("Failed to upload image. Please try again.");
            } finally {
                submitButton.disabled = false;
            }
        });
    }
}