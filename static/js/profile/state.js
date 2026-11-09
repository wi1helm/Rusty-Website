/* =========================================================
   profile/state.js  (EDITOR only)
   The editable profile data, shared by all editor modules.
   Each module's init function fills in its own part.
========================================================= */

import { createGraph } from "../shared/graph.js";

export const state = {
    displayName: "",
    about: "",
    links: [],
    graph: createGraph()
};