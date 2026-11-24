import {createClientId} from "../helpers.js";

/*
 * shared/graph-editor.js
 * Shared graph structure and data helpers
 */

export function createGraph() {
    return {
        positions: [],
        connections: []
    };
}

/*
 * Read initial graph data from the HTML document.
 */
export function getGraph(root = document) {
    const element = root.querySelector("#organization-graph-data");

    if (!element) {
        console.warn("Initial organization graph data was not found.");
        return createGraph();
    }

    try {
        return hydrateGraph(JSON.parse(element.textContent));
    } catch (error) {
        console.error("Failed to parse organization graph data:", error);
        return createGraph();
    }
}

/*
 * Find the currently active status record.
 */
export function getCurrentStatus(position) {
    if (!position?.statuses) {
        return null;
    }

    return position.statuses.find(
        status => status.id === position.current_status_id
    ) ?? null;
}

/*
 * Find a particular status record.
 */
export function getStatus(position, statusName) {
    return position?.statuses?.find(
        status => status.status === statusName
    ) ?? null;
}

/*
 * Create a new position.
 */
export function createPosition(values = {}) {
    const statusId = values.current_status_id ?? createClientId();

    const status = values.status ?? "open";

    return {
        // Database identity.
        // Null means this position has not been saved yet.
        id: values.id ?? null,

        // Browser/editor identity.
        client_id: values.client_id ?? createClientId(),

        title: values.title ?? "",
        description: values.description ?? "",

        current_status_id: statusId,

        statuses: values.statuses ?? [
            {
                id: statusId,
                status,
                information: values.information ?? {}
            }
        ]
    };
}

export function hydrateGraph(data) {
    const positions = (data.positions ?? []).map(position => ({
        ...position,
        client_id: createClientId()
    }));

    const clientIdsByDatabaseId = new Map(
        positions.map(position => [
            position.id,
            position.client_id
        ])
    );

    const connections = (data.connections ?? []).map(connection => ({
        ...connection,
        client_id: createClientId(),

        from: clientIdsByDatabaseId.get(connection.from),
        to: clientIdsByDatabaseId.get(connection.to)
    }));

    return {
        positions,
        connections
    };
}
