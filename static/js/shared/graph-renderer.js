import { renderConnections } from "./graph-connections.js";
import {getCurrentStatus} from "./graph.js";

export function renderGraph(graph, container) {
    if (!container) {
        console.error("Graph render container was not found.");
        return;
    }

    const positions = graph?.positions ?? [];
    const fragment = document.createDocumentFragment();

    for (const position of positions) {
        fragment.appendChild(createPositionCard(position));
    }

    container.replaceChildren(fragment);

    const canvas = container.closest(
        ".organization-graph-canvas"
    );

    if (canvas) {
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



        renderConnections(graph, canvas);

        // Redraw when the grid changes size.
        if (!container.dataset.observingSize) {
            const observer = new ResizeObserver(() => {
                renderConnections(graph, canvas);
            });

            observer.observe(container);

            container.dataset.observingSize = "true";
        }
    }

    const graphRoot = container.closest(
        ".organization-graph"
    );

    const placeholder = graphRoot?.querySelector(
        "#organization-graph-placeholder"
    );

    if (placeholder) {
        placeholder.hidden = positions.length > 0;
    }
}

function createPositionCard(position) {
    const card = document.createElement("button");

    card.type = "button";
    card.className = "organization-graph-position";

    card.dataset.positionId = position.id;

    card.setAttribute(
        "aria-label",
        `View ${position.title || "untitled position"}`
    );

    // Generic organization icon.
    const icon = document.createElement("span");

    icon.className = "organization-graph-position-icon";
    icon.setAttribute("aria-hidden", "true");

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

    // Position title.
    const title = document.createElement("h3");

    title.className = "organization-graph-position-title";
    title.textContent = position.title || "Untitled position";

    // Small loading placeholder.
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

    statusLabel.setAttribute("aria-label", `Status: ${statusLabel.textContent}`);

    card.append(statusLabel, icon, title, skeleton);

    return card;
}