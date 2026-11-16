const SVG_NS = "http://www.w3.org/2000/svg";

export function renderConnections(layout, canvas) {
    if (!canvas || !layout) {
        return;
    }

    const svg = canvas.querySelector(
        ".organization-graph-connections"
    );

    if (!svg) {
        return;
    }

    svg.replaceChildren();

    const width = Math.max(layout.width, 1);
    const height = Math.max(layout.height, 1);

    svg.style.width = `${width}px`;
    svg.style.height = `${height}px`;

    svg.setAttribute("width", width);
    svg.setAttribute("height", height);
    svg.setAttribute("viewBox", `0 0 ${width} ${height}`);

    // Find the calculated position of every card.
    const nodesById = new Map(
        layout.nodes.map(node => [node.id, node])
    );

    for (const edge of layout.edges) {
        const source = nodesById.get(edge.sources[0]);
        const target = nodesById.get(edge.targets[0]);

        if (!source || !target) {
            continue;
        }

        // Bottom centre of the parent.
        const start = {
            x: source.x + source.width / 2,
            y: source.y + source.height
        };

        // Top centre of the child.
        const end = {
            x: target.x + target.width / 2,
            y: target.y
        };

        // Horizontal branching line between the cards.
        const middleY = (start.y + end.y) / 2;

        const points = [
            start,
            { x: start.x, y: middleY },
            { x: end.x, y: middleY },
            end
        ];

        const path = document.createElementNS(
            SVG_NS,
            "path"
        );

        path.classList.add(
            "organization-graph-connection"
        );

        path.setAttribute(
            "d",
            createRoundedPath(points, 8)
        );

        svg.appendChild(path);
    }
}


/*
 * Convert straight orthogonal points into
 * a path with rounded corners.
 */
function createRoundedPath(points, radius) {
    if (points.length < 2) {
        return "";
    }

    let path = `M ${points[0].x} ${points[0].y}`;

    for (let i = 1; i < points.length - 1; i++) {
        const previous = points[i - 1];
        const current = points[i];
        const next = points[i + 1];

        const incomingLength = Math.hypot(
            current.x - previous.x,
            current.y - previous.y
        );

        const outgoingLength = Math.hypot(
            next.x - current.x,
            next.y - current.y
        );

        const incoming = Math.min(radius, incomingLength / 2);
        const outgoing = Math.min(radius, outgoingLength / 2);

        const before = {
            x: current.x +
                Math.sign(previous.x - current.x) * incoming,
            y: current.y +
                Math.sign(previous.y - current.y) * incoming
        };

        const after = {
            x: current.x +
                Math.sign(next.x - current.x) * outgoing,
            y: current.y +
                Math.sign(next.y - current.y) * outgoing
        };

        path += ` L ${before.x} ${before.y}`;
        path += ` Q ${current.x} ${current.y} ${after.x} ${after.y}`;
    }

    const last = points[points.length - 1];

    path += ` L ${last.x} ${last.y}`;

    return path;
}