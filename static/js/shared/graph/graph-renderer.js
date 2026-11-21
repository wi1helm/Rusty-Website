import { renderConnections } from "./graph-connections.js";
import { layoutGraph } from "./graph-layout.js";
import { getCurrentStatus } from "./graph.js";

export async function renderGraph(graph, container) {
    if (!container) {
        console.error("Graph render container was not found.");
        return;
    }

    const positions = graph?.positions ?? [];
    const fragment = document.createDocumentFragment();

    for (const position of positions) {
        fragment.appendChild(createPositionCard(position));
    }
    container.classList.add("is-layout-pending");
    container.replaceChildren(fragment);

    const canvas = container.closest(
        ".organization-graph-canvas"
    );

    const graphRoot = container.closest(
        ".organization-graph"
    );

    const placeholder = graphRoot?.querySelector(
        "#organization-graph-placeholder"
    );

    if (placeholder) {
        placeholder.hidden = positions.length > 0;
    }

    if (!canvas || positions.length === 0) {
        container.style.width = "";
        container.style.height = "";

        canvas?.querySelector(
            ".organization-graph-connections"
        )?.replaceChildren();

        return;
    }

    let svg = canvas.querySelector(
        ".organization-graph-connections"
    );

    if (!svg) {
        svg = document.createElementNS(
            "http://www.w3.org/2000/svg",
            "svg"
        );

        svg.classList.add(
            "organization-graph-connections"
        );

        svg.setAttribute("aria-hidden", "true");

        canvas.prepend(svg);
    }

    // Wait for the browser to calculate the cards' dimensions.
    await new Promise(resolve =>
        requestAnimationFrame(resolve)
    );

    const layout = await layoutGraph(graph, container);

    // Apply ELK's calculated positions.
    const nodesById = new Map(
        layout.nodes.map(node => [node.id, node])
    );

    for (const card of container.children) {
        const node = nodesById.get(card.dataset.positionId);

        if (!node) {
            continue;
        }

        card.style.position = "absolute";
        card.style.left = `${node.x}px`;
        card.style.top = `${node.y}px`;
    }

    container.style.position = "relative";
    container.style.width = `${layout.width}px`;
    container.style.height = `${layout.height}px`;

    // Draw the connection routes calculated by ELK.
    renderConnections(layout, canvas);

    container.classList.remove("is-layout-pending");
}

function createPositionCard(position) {
    const card = document.createElement("button");

    card.type = "button";
    card.className = "organization-graph-position";

    // Use the browser identity, not the database identity.
    card.dataset.positionId = position.client_id;

    card.setAttribute(
        "aria-label",
        `View ${position.title || "untitled position"}`
    );

    const icon = document.createElement("span");

    icon.className = "organization-graph-position-icon";
    icon.setAttribute("aria-hidden", "true");

    // TODO: innerHTML is used to render SVG content. While the current SVG is hardcoded and safe, this pattern is fragile — if the SVG source is ever parameterized (e.g., different icons per status), it becomes an immediate cross-site scripting (XSS) vector where an attacker could inject malicious JavaScript. Consider using document.createElementNS() and setAttribute() to build SVG elements safely instead of innerHTML.
    icon.innerHTML = `
        <svg
            xmlns="http://www.w3.org/2000/svg"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-linecap="round"
            stroke-linejoin="round"
        >
            <rect x="3" y="4" width="18" height="16" rx="2" />
            <path d="M9 4v16" />
            <path d="M15 4v16" />
            <path d="M6 8h.01" />
            <path d="M6 12h.01" />
            <path d="M6 16h.01" />
            <path d="M12 8h.01" />
            <path d="M12 12h.01" />
            <path d="M12 16h.01" />
            <path d="M18 8h.01" />
            <path d="M18 12h.01" />
            <path d="M18 16h.01" />
        </svg>
    `;

    const title = document.createElement("h3");

    title.className = "organization-graph-position-title";
    title.textContent = position.title || "Untitled position";

    const skeleton = document.createElement("div");

    skeleton.className = "position-card-skeleton";
    skeleton.setAttribute("aria-hidden", "true");

    const line = document.createElement("span");

    line.className = "position-card-skeleton-line";

    skeleton.appendChild(line);

    const status = getCurrentStatus(position);

    const statusLabel = document.createElement("span");

    statusLabel.className = "organization-graph-position-status";

    statusLabel.textContent = (
        status?.status ?? "unknown"
    ).toUpperCase();

    statusLabel.dataset.status = status?.status ?? "unknown";

    statusLabel.setAttribute(
        "aria-label",
        `Status: ${statusLabel.textContent}`
    );

    card.append(statusLabel, icon, title, skeleton);

    return card;
}