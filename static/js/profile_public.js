/* =========================================================
   profile_public.js  (PUBLIC page entry)
   Renders public profile links and organization graph.
========================================================= */

import { renderLinkIcon } from "./shared/link-icons.js";
import { getGraph } from "./shared/graph/graph.js";
import { renderGraph } from "./shared/graph/graph-renderer.js";

document.querySelectorAll(".profile-link-icon[data-url]").forEach(element => {
    renderLinkIcon(element, element.dataset.url);
});

const graphPositions = document.querySelector(".organization-graph-positions");
if (graphPositions) {
    renderGraph(getGraph(), graphPositions);
}