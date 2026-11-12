import { openDialog, closeDialog } from "./dialogs.js";

import {
    getGraph,
    createPosition,
    getCurrentStatus, getTestGraph
} from "../shared/graph.js";

import {renderGraph} from "../shared/graph-renderer.js";


export function initGraph() {
    const positionsContainer = document.querySelector(".organization-graph-positions");

    const placeholder = document.querySelector("#organization-graph-placeholder");

    const addButton = document.querySelector("#add-position-button");

    const dialog = document.querySelector("#position-dialog");

    const dialogTitle = document.querySelector("#position-dialog-title");

    const form = document.querySelector("#position-form");

    const statusSelect = form?.elements.namedItem("status");

    if (
        !positionsContainer ||
        !placeholder ||
        !addButton ||
        !dialog ||
        !dialogTitle ||
        !form ||
        !statusSelect
    ) {
        console.warn("Could not initialize organization graph.");
        return;
    }

    const state = {
        graph: getTestGraph()
    };

    let editingPosition = null;

    // Holds unsaved information while switching between statuses.
    let statusDrafts = {};


    /*
     * Initial rendering
     */

    renderGraph(state.graph, positionsContainer);
    updateGraphEmptyState(state.graph, placeholder);


    /*
     * Add position
     */

    addButton.addEventListener("click", () => {
        editingPosition = null;
        statusDrafts = {};

        openNewPositionDialog(
            form,
            dialog,
            dialogTitle,
            statusSelect
        );
    });


    /*
     * Status selection
     */

    statusSelect.addEventListener("change", () => {
        const previousStatus = statusSelect.dataset.previousStatus;
        const selectedStatus = statusSelect.value;

        // Save the previous status information.
        if (previousStatus) {
            saveStatusDraft(
                form,
                previousStatus,
                statusDrafts
            );
        }

        // Show the selected status fields.
        updateStatusFields(selectedStatus);

        statusSelect.dataset.previousStatus = selectedStatus;
    });


    /*
     * Edit position
     */

    positionsContainer.addEventListener("click", event => {
        const box = event.target.closest(
            ".organization-graph-position"
        );

        if (!box) {
            return;
        }

        const position = state.graph.positions.find(
            item => item.id === box.dataset.positionId
        );

        if (!position) {
            return;
        }

        editingPosition = position;
        statusDrafts = {};

        openEditPositionDialog(
            position,
            form,
            dialog,
            dialogTitle,
            statusSelect,
            statusDrafts
        );
    });


    /*
     * Submit position form
     */

    form.addEventListener("submit", event => {
        event.preventDefault();

        // Save the currently selected status information.
        saveStatusDraft(
            form,
            statusSelect.value,
            statusDrafts
        );

        const position = getPositionFromForm(
            form,
            editingPosition,
            statusDrafts
        );

        if (editingPosition) {
            const index = state.graph.positions.findIndex(
                item => item.id === editingPosition.id
            );

            if (index !== -1) {
                state.graph.positions[index] = position;
            }
        } else {
            state.graph.positions.push(position);
        }
        renderGraph(state.graph, positionsContainer);
        updateGraphEmptyState(state.graph, placeholder);

        closeDialog(dialog);

        editingPosition = null;
        statusDrafts = {};
    });
}


function updateGraphEmptyState(graph, placeholder) {
    placeholder.hidden = graph.positions.length > 0;
}

/*
 * Form helpers
 */

function getField(form, name) {
    const field = form.elements.namedItem(name);

    if (!field) {
        console.warn(`Form field "${name}" was not found.`);
        return "";
    }

    return field.value;
}


function setField(form, name, value) {
    const field = form.elements.namedItem(name);

    if (!field) {
        console.warn(`Form field "${name}" was not found.`);
        return;
    }

    field.value = value ?? "";
}


/*
 * Status helpers
 */

const POSITION_STATUSES = [
    "open",
    "closed",
    "archived",
    "filled"
];


function updateStatusFields(selectedStatus) {
    document.querySelectorAll("[data-status-fields]").forEach(section => {
        section.hidden =
            section.dataset.statusFields !== selectedStatus;
    });
}


function saveStatusDraft(form, status, drafts) {
    if (!POSITION_STATUSES.includes(status)) {
        return;
    }

    const field = form.elements.namedItem(
        `${status}.information`
    );

    if (!field) {
        return;
    }

    drafts[status] = {
        information: field.value
    };
}


function loadStatusInformation(form, position, drafts) {
    const statuses = position?.statuses ?? [];

    for (const status of POSITION_STATUSES) {
        const field = form.elements.namedItem(
            `${status}.information`
        );

        if (!field) {
            continue;
        }

        const statusRecord = statuses.find(
            item => item.status === status
        );

        const information =
            drafts[status]?.information ??
            statusRecord?.information?.information ??
            statusRecord?.information ??
            "";

        field.value =
            typeof information === "string"
                ? information
                : "";
    }
}


/*
 * Dialog helpers
 */

function openNewPositionDialog(form, dialog, dialogTitle, statusSelect) {
    form.reset();

    dialogTitle.textContent = "Add position";

    statusSelect.value = "open";
    statusSelect.dataset.previousStatus = "open";

    updateStatusFields("open");

    openDialog(dialog);
}


function openEditPositionDialog(position, form, dialog, dialogTitle, statusSelect, drafts) {
    form.reset();

    dialogTitle.textContent = "Edit position";

    setField(form, "title", position.title);
    setField(form, "description", position.description);

    const currentStatus = getCurrentStatus(position);

    if (!currentStatus) {
        console.error(
            "Position has no current status:",
            position
        );

        return;
    }

    statusSelect.value = currentStatus.status;
    statusSelect.dataset.previousStatus = currentStatus.status;

    loadStatusInformation(
        form,
        position,
        drafts
    );

    updateStatusFields(currentStatus.status);

    openDialog(dialog);
}


/*
 * Build position from form
 */

function getPositionFromForm(form, existingPosition, drafts) {
    const selectedStatus = getField(form, "status");

    const title = getField(form, "title");
    const description = getField(form, "description");

    const statuses = existingPosition
        ? [...(existingPosition.statuses ?? [])]
        : [];

    // Get the selected status information.
    const information = drafts[selectedStatus] ?? {
        information: ""
    };

    // Find the selected status record.
    let statusRecord = statuses.find(
        item => item.status === selectedStatus
    );

    if (statusRecord) {
        // Update the existing status record.
        statusRecord = {
            ...statusRecord,
            information
        };

        const index = statuses.findIndex(
            item => item.status === selectedStatus
        );

        statuses[index] = statusRecord;
    } else {
        // Create a new status record.
        statusRecord = {
            id: crypto.randomUUID(),
            status: selectedStatus,
            information
        };

        statuses.push(statusRecord);
    }

    // Create a new position.
    if (!existingPosition) {
        return {
            ...createPosition({
                title,
                description,
                status: selectedStatus,
                information
            }),
            statuses: [statusRecord],
            current_status_id: statusRecord.id
        };
    }

    // Update an existing position.
    return {
        ...existingPosition,
        title,
        description,
        current_status_id: statusRecord.id,
        statuses
    };
}