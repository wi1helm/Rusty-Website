import assert from "node:assert/strict";
import test from "node:test";
import { getGraph } from "./graph.js";

test("getGraph reads initial JSON from the hidden data container", () => {
    const root = {
        querySelector(selector) {
            assert.equal(selector, "#organization-graph-data");
            return { textContent: '{"positions":[],"connections":[]}' };
        }
    };

    assert.deepEqual(getGraph(root), { positions: [], connections: [] });
});