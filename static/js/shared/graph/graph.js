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
 * Read the initial graph and append temporary test data.
 */
export function getTestGraph(root = document) {
    const graph = getGraph(root);
    const testGraph = createTestGraph();

    graph.positions.push(...testGraph.positions);
    graph.connections.push(...testGraph.connections);

    return graph;
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
    // TODO: crypto.randomUUID() is called without checking if it's available. In non-secure (HTTP) contexts, the crypto API may be unavailable, causing an uncaught exception when creating new positions. Add a fallback for generating unique IDs.
    const statusId = values.current_status_id ?? crypto.randomUUID();

    const status = values.status ?? "open";

    return {
        // Database identity.
        // Null means this position has not been saved yet.
        id: values.id ?? null,

        // Browser/editor identity.
        client_id: values.client_id ?? crypto.randomUUID(),

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
    // TODO: crypto.randomUUID() is called without checking if it's available. In non-secure (HTTP) contexts, the crypto API may be unavailable, causing an uncaught exception when hydrating graph data from the server. Add a fallback for generating unique IDs.
    const positions = (data.positions ?? []).map(position => ({
        ...position,
        client_id: crypto.randomUUID()
    }));

    const clientIdsByDatabaseId = new Map(
        positions.map(position => [
            position.id,
            position.client_id
        ])
    );

    const connections = (data.connections ?? []).map(connection => ({
        ...connection,
        client_id: crypto.randomUUID(),

        from: clientIdsByDatabaseId.get(connection.from),
        to: clientIdsByDatabaseId.get(connection.to)
    }));

    return {
        positions,
        connections
    };
}

/*
 * Temporary test data for styling the organization graph.
 */
export function createTestGraph() {
    const positions = [
        createPosition({
            client_id: "position-ceo",
            title: "Chief Executive Officer",
            description: "Leading the company, setting the vision and keeping everyone moving in the same direction.",
            status: "open",
            information: {
                information: "Responsible for the overall direction and strategy of the organization."
            }
        }),

        createPosition({
            client_id: "position-cto",
            title: "Chief Technology Officer",
            description: "Oversees technology, engineering and product development.",
            status: "filled",
            information: {
                information: "Currently held by Alex Morgan."
            }
        }),

        createPosition({
            client_id: "position-cfo",
            title: "Chief Financial Officer",
            description: "Responsible for financial planning, budgeting and reporting.",
            status: "filled",
            information: {
                information: "Currently held by Jamie Andersson."
            }
        }),

        createPosition({
            client_id: "position-design",
            title: "Head of Design",
            description: "Shapes the user experience and visual direction of our products.",
            status: "open",
            information: {
                information: "We're looking for someone with strong product design experience."
            }
        }),

        createPosition({
            client_id: "position-backend",
            title: "Backend Developer",
            description: "Builds and maintains APIs, databases and backend services.",
            status: "open",
            information: {
                information: "Experience with Rust and PostgreSQL is a plus."
            }
        }),

        createPosition({
            client_id: "position-frontend",
            title: "Frontend Developer",
            description: "Creates accessible, responsive and enjoyable user interfaces.",
            status: "filled",
            information: {
                information: "Currently held by Taylor Smith."
            }
        }),

        createPosition({
            client_id: "position-marketing",
            title: "Marketing Manager",
            description: "Plans campaigns, manages communication and grows brand awareness.",
            status: "closed",
            information: {
                information: "Applications are currently closed."
            }
        }),

        createPosition({
            client_id: "position-intern",
            title: "Design Intern",
            description: "Works alongside the design team on everyday projects and experiments.",
            status: "archived",
            information: {
                information: "This position is no longer active."
            }
        })
    ];

    return {
        positions,
        connections: [
            {
                client_id: "connection-ceo-cto",
                from: "position-ceo",
                to: "position-cto"
            },
            {
                client_id: "connection-ceo-cfo",
                from: "position-ceo",
                to: "position-cfo"
            },
            {
                client_id: "connection-cto-backend",
                from: "position-cto",
                to: "position-backend"
            },
            {
                client_id: "connection-cto-frontend",
                from: "position-cto",
                to: "position-frontend"
            }
        ]
    };
}