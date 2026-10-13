/* =========================================================
   profile_public.js  (PUBLIC page entry)
   Upgrades the server-rendered generic link icons.
========================================================= */

import { renderLinkIcon } from "./shared/link-icons.js";

document.querySelectorAll(".profile-link-icon[data-url]").forEach(element => {
    renderLinkIcon(element, element.dataset.url);
});