/* =========================================================
   profile/buttons.js  (EDITOR only)
========================================================= */

import { createElement, createSvgIcon, icons } from "../shared/helpers.js";

export function makeButton(label, iconName, className) {
    const button = createElement("button", className);

    button.type = "button";
    button.setAttribute("aria-label", label);
    button.title = label;

    if (icons[iconName]) {
        button.appendChild(createSvgIcon(icons[iconName], 15));
    }

    return button;
}