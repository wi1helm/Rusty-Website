document.addEventListener("DOMContentLoaded", () => {
    const modal = document.getElementById("image-upload-modal");
    const input = document.getElementById("image-upload-input");
    const preview = document.getElementById("image-upload-preview");
    if (!modal || !input || !preview) {
        console.error("Profile image upload UI is missing.");
        return;
    }
    const previewImage = preview.querySelector("img");

    const title = document.getElementById("image-upload-title");
    const description = document.getElementById("image-upload-description");

    const closeButton = document.querySelector(".image-upload-close");
    const cancelButton = document.getElementById("image-upload-cancel");
    const submitButton = document.getElementById("image-upload-submit");

    const dropzone = document.querySelector(".image-upload-dropzone");

    let currentImageType = null;
    let currentUploadUrl = null;
    let selectedFile = null;


    /*
     * Open editor
     */

    document.querySelectorAll(".profile-image-editor").forEach(editor => {

        editor.addEventListener("click", () => {

            currentImageType = editor.dataset.imageType;
            currentUploadUrl = editor.dataset.uploadUrl;

            selectedFile = null;

            input.value = "";
            preview.hidden = true;
            previewImage.src = "";

            if (currentImageType === "logo") {
                title.textContent = "Upload logo";
                description.textContent =
                    "Choose a square image. Recommended size: 400 × 400 pixels.";
            } else if (currentImageType === "banner") {
                title.textContent = "Upload banner";
                description.textContent =
                    "Choose a banner image. Recommended size: 1500 × 500 pixels.";
            }

            modal.hidden = false;
        });

    });


    /*
     * Close editor
     */

    function closeModal() {
        modal.hidden = true;

        selectedFile = null;
        currentImageType = null;
        currentUploadUrl = null;

        input.value = "";

        preview.hidden = true;
        previewImage.src = "";
    }

    closeButton.addEventListener("click", closeModal);
    cancelButton.addEventListener("click", closeModal);


    /*
     * Clicking the dark area outside the dialog closes it.
     */

    modal.addEventListener("click", event => {

        if (event.target === modal) {
            closeModal();
        }

    });


    /*
     * Escape closes the modal.
     */

    document.addEventListener("keydown", event => {

        if (event.key === "Escape" && !modal.hidden) {
            closeModal();
        }

    });


    /*
     * File selection
     */

    input.addEventListener("change", () => {

        if (input.files.length === 0) {
            return;
        }

        selectFile(input.files[0]);

    });


    function selectFile(file) {

        if (!file.type.startsWith("image/")) {
            alert("Please select an image.");
            return;
        }

        selectedFile = file;

        const url = URL.createObjectURL(file);

        previewImage.src = url;
        preview.hidden = false;

    }


    /*
     * Drag & drop
     */

    dropzone.addEventListener("dragover", event => {

        event.preventDefault();

        dropzone.classList.add("dragging");

    });


    dropzone.addEventListener("dragleave", () => {

        dropzone.classList.remove("dragging");

    });


    dropzone.addEventListener("drop", event => {

        event.preventDefault();

        dropzone.classList.remove("dragging");

        const files = event.dataTransfer.files;

        if (files.length === 0) {
            return;
        }

        selectFile(files[0]);

    });


    /*
     * Upload
     */

    submitButton.addEventListener("click", async () => {

        if (!selectedFile) {
            alert("Please choose an image first.");
            return;
        }

        submitButton.disabled = true;

        try {

            const formData = new FormData();

            formData.append("image", selectedFile);
            formData.append("image_type", currentImageType);

            const response = await fetch(currentUploadUrl, {
                method: "POST",
                body: formData,
            });

            if (!response.ok) {
                throw new Error("Upload failed.");
            }

            closeModal();

            /*
             * For now, reload the page so the newly uploaded
             * image appears.
             *
             * Later we can make this update the image without
             * reloading.
             */

            window.location.reload();

        } catch (error) {

            console.error(error);

            alert("Failed to upload image.");

        } finally {

            submitButton.disabled = false;

        }

    });

});