/* =========================================================
   profile/state.js  (EDITOR only)
   The editable profile data, shared by all editor modules.
   Each module's init function fills in its own part.
========================================================= */

export const state = {
    displayName: "",
    about: "",
    links: []   // [{ id, label, url }]
};