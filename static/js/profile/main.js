/* =========================================================
   profile/main.js  (EDITOR entry point)
   Load with: <script type="module" src="/assets/js/profile/main.js">
   Module scripts are deferred, so the DOM is ready here.
========================================================= */

import { state } from "./state.js";

import { initDialogs } from "./dialogs.js";
import { initDisplayName } from "./display-name.js";
import { initAbout } from "./about.js";
import { initLinks } from "./links.js";
import { initSave } from "./save.js";
import { initImageUpload } from "./image-upload.js";
import { initGraph } from "./graph.js";

initDialogs();
initDisplayName(state);
initAbout(state);
initLinks(state);
initSave(state);
initImageUpload();

initGraph(state);