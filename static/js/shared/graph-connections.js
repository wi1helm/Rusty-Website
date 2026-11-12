const SVG_NS = "http://www.w3.org/2000/svg";

export function renderConnections(graph, canvas) {
    if (!canvas) {
        return;
    }

    const svg = canvas.querySelector(
        ".organization-graph-connections"
    );

    if (!svg) {
        return;
    }

    svg.replaceChildren();

    const width = canvas.scrollWidth;
    const height = canvas.scrollHeight;

    svg.style.width = `${width}px`;
    svg.style.height = `${height}px`;

    svg.setAttribute(
        "viewBox",
        `0 0 ${width} ${height}`
    );

    const cards = new Map();

    canvas.querySelectorAll(
        ".organization-graph-position"
    ).forEach(card => {
        const rect = card.getBoundingClientRect();
        const canvasRect = canvas.getBoundingClientRect();

        cards.set(card.dataset.positionId, {
            left: rect.left - canvasRect.left,
            right: rect.right - canvasRect.left,
            top: rect.top - canvasRect.top,
            bottom: rect.bottom - canvasRect.top,
            centerX: rect.left - canvasRect.left + rect.width / 2,
            centerY: rect.top - canvasRect.top + rect.height / 2
        });
    });

    for (const connection of graph.connections ?? []) {
        const from = cards.get(connection.from);
        const to = cards.get(connection.to);

        if (!from || !to) {
            continue;
        }

        const path = document.createElementNS(
            SVG_NS,
            "path"
        );

        path.classList.add("organization-graph-connection");

        path.setAttribute(
            "d",
            createConnectionPath(from, to)
        );

        svg.appendChild(path);
    }
}


/*
 * Create an orthogonal connection.
 *
 * Every segment is horizontal or vertical.
 * Corners are rounded using quadratic curves.
 */

function createConnectionPath(from, to) {
    let points;

    if (to.centerY > from.centerY) {
        // Target is below the source.

        const middleY = (from.bottom + to.top) / 2;

        points = [
            { x: from.centerX, y: from.bottom },
            { x: from.centerX, y: middleY },
            { x: to.centerX, y: middleY },
            { x: to.centerX, y: to.top }
        ];

    } else if (to.centerY < from.centerY) {
        // Target is above the source.

        const middleY = (from.top + to.bottom) / 2;

        points = [
            { x: from.centerX, y: from.top },
            { x: from.centerX, y: middleY },
            { x: to.centerX, y: middleY },
            { x: to.centerX, y: to.bottom }
        ];

    } else {
        // Cards are on the same row.
        // Route above them to avoid crossing cards.

        const middleY = Math.min(from.top, to.top) - 24;

        points = [
            { x: from.centerX, y: from.top },
            { x: from.centerX, y: middleY },
            { x: to.centerX, y: middleY },
            { x: to.centerX, y: to.top }
        ];
    }

    return createRoundedPath(points, 8);
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

        const incoming = Math.min(
            radius,
            Math.abs(current.x - previous.x) / 2 ||
            Math.abs(current.y - previous.y) / 2
        );

        const outgoing = Math.min(
            radius,
            Math.abs(next.x - current.x) / 2 ||
            Math.abs(next.y - current.y) / 2
        );

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