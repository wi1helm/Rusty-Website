import { openDialog, closeDialog } from "./dialogs.js";

import {
    createPosition,
    getCurrentStatus
} from "../shared/graph/graph.js";

import { renderGraph } from "../shared/graph/graph-renderer.js";


/*
 * Configuration
 */

const POSITION_STATUSES = [
    "open",
    "closed",
    "archived",
    "filled"
];


/*
 * Initialization
 */

export function initGraph(state) {
    const elements = {
        positionsContainer: document.querySelector(
            ".organization-graph-positions"
        ),
        placeholder: document.querySelector(
            "#organization-graph-placeholder"
        ),
        addButton: document.querySelector(
            "#add-position-button"
        ),
        connectButton: document.querySelector(
            "#connect-position-button"
        ),
        dialog: document.querySelector(
            "#position-dialog"
        ),
        dialogTitle: document.querySelector(
            "#position-dialog-title"
        ),
        form: document.querySelector(
            "#position-form"
        )
    };

    elements.statusSelect =
        elements.form?.elements.namedItem("status");

    if (
        !elements.positionsContainer ||
        !elements.placeholder ||
        !elements.addButton ||
        !elements.connectButton ||
        !elements.dialog ||
        !elements.dialogTitle ||
        !elements.form ||
        !elements.statusSelect
    ) {
        console.warn("Could not initialize organization graph.");
        return;
    }

    const editor = {
        state,
        elements,

        editingPosition: null,
        statusDrafts: {},

        connecting: false,
        connectionSource: null
    };

    renderEditorGraph(editor);

    bindAddPosition(editor);
    bindConnectionMode(editor);
    bindPositionSelection(editor);
    bindStatusSelection(editor);
    bindPositionForm(editor);
}


/*
 * Rendering
 */

function renderEditorGraph(editor) {
    const { state, elements } = editor;

    renderGraph(
        state.graph,
        elements.positionsContainer
    );

    elements.placeholder.hidden =
        state.graph.positions.length > 0;

    // Restore selected position after rendering.
    if (editor.connectionSource) {
        const card = findPositionCard(
            editor,
            editor.connectionSource.client_id
        );

        card?.classList.add("is-connection-source");
    }

    updateConnectionModeUI(editor);
}


/*
 * Add position
 */

function bindAddPosition(editor) {
    const { elements } = editor;

    elements.addButton.addEventListener("click", () => {
        cancelConnectionMode(editor);

        editor.editingPosition = null;
        editor.statusDrafts = {};

        openNewPositionDialog(
            elements.form,
            elements.dialog,
            elements.dialogTitle,
            elements.statusSelect
        );
    });
}


/*
 * Connection mode
 */

function bindConnectionMode(editor) {
    const { connectButton } = editor.elements;

    connectButton.addEventListener("click", () => {
        if (editor.connecting) {
            cancelConnectionMode(editor);
            return;
        }

        editor.connecting = true;
        editor.connectionSource = null;

        updateConnectionModeUI(editor);
    });

    document.addEventListener("keydown", event => {
        if (
            event.key === "Escape" &&
            editor.connecting
        ) {
            cancelConnectionMode(editor);
        }
    });
}


function handleConnectionSelection(editor, position) {
    // First click: select the manager.
    if (!editor.connectionSource) {
        editor.connectionSource = position;

        findPositionCard(
            editor,
            position.client_id
        )?.classList.add("is-connection-source");

        updateConnectionModeUI(editor);
        return;
    }

    const source = editor.connectionSource;
    const target = position;

    // Clicking the same position deselects it.
    if (source.client_id === target.client_id) {
        editor.connectionSource = null;

        findPositionCard(
            editor,
            source.client_id
        )?.classList.remove("is-connection-source");

        updateConnectionModeUI(editor);
        return;
    }

    toggleConnection(editor, source, target);
}


function toggleConnection(editor, source, target) {
    const { graph } = editor.state;

    const existingConnection = graph.connections.find(
        connection =>
            connection.from === source.client_id &&
            connection.to === target.client_id
    );

    // Existing connection: remove it.
    if (existingConnection) {
        graph.connections = graph.connections.filter(
            connection => connection !== existingConnection
        );

        finishConnectionSelection(editor);
        renderEditorGraph(editor);
        return;
    }

    // Prevent duplicate relationships in the opposite direction.
    const reverseConnection = graph.connections.some(
        connection =>
            connection.from === target.client_id &&
            connection.to === source.client_id
    );

    if (reverseConnection) {
        console.warn(
            "These positions are already connected in the opposite direction."
        );
        return;
    }

    // Prevent circular reporting relationships.
    if (
        wouldCreateCycle(
            graph,
            source.client_id,
            target.client_id
        )
    ) {
        console.warn(
            "This connection would create a reporting cycle."
        );
        return;
    }

    graph.connections.push({
        // TODO: crypto.randomUUID() is called without checking if it's available. In non-secure (HTTP) contexts, the crypto API may be unavailable, causing an uncaught exception that breaks the connection feature. Add a fallback similar to createLinkId() in links.js.
        client_id: crypto.randomUUID(),
        from: source.client_id,
        to: target.client_id
    });

    finishConnectionSelection(editor);
    renderEditorGraph(editor);
}


function wouldCreateCycle(graph, sourceId, targetId) {
    // Adding source -> target creates a cycle if
    // target can already reach source.

    const visited = new Set();
    const pending = [targetId];

    while (pending.length > 0) {
        const current = pending.pop();

        if (current === sourceId) {
            return true;
        }

        if (visited.has(current)) {
            continue;
        }

        visited.add(current);

        for (const connection of graph.connections) {
            if (connection.from === current) {
                pending.push(connection.to);
            }
        }
    }

    return false;
}


function finishConnectionSelection(editor) {
    editor.connectionSource = null;

    updateConnectionModeUI(editor);
}


function cancelConnectionMode(editor) {
    editor.connecting = false;
    editor.connectionSource = null;

    editor.elements.positionsContainer
        .querySelectorAll(".is-connection-source")
        .forEach(card => {
            card.classList.remove("is-connection-source");
        });

    updateConnectionModeUI(editor);
}


function updateConnectionModeUI(editor) {
    const { connectButton } = editor.elements;

    connectButton.classList.toggle(
        "is-active",
        editor.connecting
    );

    connectButton.setAttribute(
        "aria-pressed",
        String(editor.connecting)
    );

    if (!editor.connecting) {
        connectButton.textContent = "↗ Connect positions";
    } else if (editor.connectionSource) {
        connectButton.textContent = "Select reporting position...";
    } else {
        connectButton.textContent = "Select manager...";
    }
}


/*
 * Position selection
 */

function bindPositionSelection(editor) {
    const { positionsContainer } = editor.elements;

    positionsContainer.addEventListener("click", event => {
        const card = event.target.closest(
            ".organization-graph-position"
        );

        if (!card) {
            return;
        }

        const position = editor.state.graph.positions.find(
            item => item.client_id === card.dataset.positionId
        );

        if (!position) {
            return;
        }

        // In connection mode, clicks select positions.
        if (editor.connecting) {
            handleConnectionSelection(editor, position);
            return;
        }

        openPositionEditor(editor, position);
    });
}


function openPositionEditor(editor, position) {
    const { elements } = editor;

    editor.editingPosition = position;
    editor.statusDrafts = {};

    openEditPositionDialog(
        position,
        elements.form,
        elements.dialog,
        elements.dialogTitle,
        elements.statusSelect,
        editor.statusDrafts
    );
}


function findPositionCard(editor, clientId) {
    return [
        ...editor.elements.positionsContainer.children
    ].find(
        card => card.dataset.positionId === clientId
    );
}


/*
 * Status selection
 */

function bindStatusSelection(editor) {
    const { form, statusSelect } = editor.elements;

    statusSelect.addEventListener("change", () => {
        const previousStatus =
            statusSelect.dataset.previousStatus;

        const selectedStatus = statusSelect.value;

        if (previousStatus) {
            saveStatusDraft(
                form,
                previousStatus,
                editor.statusDrafts
            );
        }

        updateStatusFields(selectedStatus);

        statusSelect.dataset.previousStatus =
            selectedStatus;
    });
}


function updateStatusFields(selectedStatus) {
    document.querySelectorAll("[data-status-fields]")
        .forEach(section => {
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
 * Position dialog
 */

function openNewPositionDialog(
    form,
    dialog,
    dialogTitle,
    statusSelect
) {
    form.reset();

    dialogTitle.textContent = "Add position";

    statusSelect.value = "open";
    statusSelect.dataset.previousStatus = "open";

    updateStatusFields("open");

    openDialog(dialog);
}


function openEditPositionDialog(
    position,
    form,
    dialog,
    dialogTitle,
    statusSelect,
    drafts
) {
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
    statusSelect.dataset.previousStatus =
        currentStatus.status;

    loadStatusInformation(
        form,
        position,
        drafts
    );

    updateStatusFields(currentStatus.status);

    openDialog(dialog);
}


/*
 * Position form
 */

function bindPositionForm(editor) {
    const { form, statusSelect, dialog } = editor.elements;

    form.addEventListener("submit", event => {
        event.preventDefault();

        saveStatusDraft(
            form,
            statusSelect.value,
            editor.statusDrafts
        );

        const position = getPositionFromForm(
            form,
            editor.editingPosition,
            editor.statusDrafts
        );

        if (editor.editingPosition) {
            const index = editor.state.graph.positions.findIndex(
                item =>
                    item.client_id ===
                    editor.editingPosition.client_id
            );

            if (index !== -1) {
                editor.state.graph.positions[index] = position;
            }
        } else {
            editor.state.graph.positions.push(position);
        }

        renderEditorGraph(editor);

        closeDialog(dialog);

        editor.editingPosition = null;
        editor.statusDrafts = {};
    });
}


function getPositionFromForm(
    form,
    existingPosition,
    drafts
) {
    const selectedStatus = getField(form, "status");

    const title = getField(form, "title");
    const description = getField(form, "description");

    // TODO: Enforce position validation in the graph API when persistence is implemented; form constraints are client-side only.

    const statuses = existingPosition
        ? [...(existingPosition.statuses ?? [])]
        : [];

    const information = drafts[selectedStatus] ?? {
        information: ""
    };

    let statusRecord = statuses.find(
        item => item.status === selectedStatus
    );

    if (statusRecord) {
        statusRecord = {
            ...statusRecord,
            information
        };

        const index = statuses.findIndex(
            item => item.status === selectedStatus
        );

        statuses[index] = statusRecord;
    } else {
        statusRecord = {
            // TODO: crypto.randomUUID() is called without checking if it's available. In non-secure (HTTP) contexts, the crypto API may be unavailable, causing an uncaught exception. Add a fallback for generating unique IDs.
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