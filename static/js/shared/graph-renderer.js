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

    const graphRoot = container.closest(".organization-graph");
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


    const title = document.createElement("h3");

    title.className = "organization-graph-position-title";
    title.textContent = position.title || "Untitled position";


    const skeleton = document.createElement("div");

    skeleton.className = "position-card-skeleton";
    skeleton.setAttribute("aria-hidden", "true");


    const lineWidths = [
        "100%",
        "78%",
        "91%",
        "65%",
        "85%"
    ];

    for (const width of lineWidths) {

        const line = document.createElement("span");

        line.className = "position-card-skeleton-line";
        line.style.width = width;

        skeleton.appendChild(line);
    }


    card.append(title, skeleton);

    return card;
}