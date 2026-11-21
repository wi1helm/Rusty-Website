/* =========================================================
   profile/state.js  (EDITOR only)
   The editable profile data, shared by all editor modules.
   Each module's init function fills in its own part.
========================================================= */

import {createGraph, createTestGraph} from "../shared/graph/graph.js";

// TODO: The state object is a plain module-level export with no immutability guards. Any script running on the page — including malicious code injected through XSS — can directly mutate state.links, state.graph, etc. without going through the save flow. Consider using getters/setters or a proxy to track changes and validate mutations.
export const state = {
    displayName: "",
    about: "",
    links: [],
    // TODO: The graph defaults to hardcoded test data (CEO, CTO, CFO positions with fake names). If the server never hydrates this with real data, the editor will show fabricated organizational data. If real data hydration is broken, users may accidentally save test data as real data. Replace with an empty graph or server-provided initial data.
    graph: createTestGraph()
};