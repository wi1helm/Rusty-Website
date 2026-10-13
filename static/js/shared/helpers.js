/* =========================================================
   shared/helpers.js  (SHARED: public + protected)
========================================================= */

export const $ = (selector, root = document) =>
    root.querySelector(selector);

export const $$ = (selector, root = document) =>
    [...root.querySelectorAll(selector)];

export function createElement(tag, className, text = null) {
    const element = document.createElement(tag);

    if (className) {
        element.className = className;
    }

    if (text !== null) {
        element.textContent = text;
    }

    return element;
}

export function createSvgIcon(paths, size = 18) {
    const ns = "http://www.w3.org/2000/svg";
    const svg = document.createElementNS(ns, "svg");

    svg.setAttribute("viewBox", "0 0 24 24");
    svg.setAttribute("width", String(size));
    svg.setAttribute("height", String(size));
    svg.setAttribute("fill", "none");
    svg.setAttribute("stroke", "currentColor");
    svg.setAttribute("stroke-width", "1.8");
    svg.setAttribute("stroke-linecap", "round");
    svg.setAttribute("stroke-linejoin", "round");
    svg.setAttribute("aria-hidden", "true");

    for (const pathData of paths) {
        const path = document.createElementNS(ns, "path");
        path.setAttribute("d", pathData);
        svg.appendChild(path);
    }

    return svg;
}

export const icons = {
    globe: [
        "M12 22a10 10 0 1 0 0-20 10 10 0 0 0 0 20Z",
        "M2 12h20",
        "M12 2a15.3 15.3 0 0 1 4 10 15.3 15.3 0 0 1-4 10",
        "M12 2a15.3 15.3 0 0 0-4 10 15.3 15.3 0 0 0 4 10"
    ],
    edit: [
        "M12 20h9",
        "M16.5 3.5a2.12 2.12 0 0 1 3 3L8 18l-4 1 1-4Z"
    ],
    trash: [
        "M3 6h18",
        "M8 6V4h8v2",
        "M19 6l-1 14H6L5 6",
        "M10 11v5",
        "M14 11v5"
    ],
    plus: [
        "M12 5v14",
        "M5 12h14"
    ],
    link: [
        "M10 13a5 5 0 0 0 7.54.54l3-3a5 5 0 0 0-7.07-7.07l-1.72 1.71",
        "M14 11a5 5 0 0 0-7.54-.54l-3 3a5 5 0 0 0 7.07 7.07l1.71-1.71"
    ]
};