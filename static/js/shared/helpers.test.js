import assert from "node:assert/strict";
import test from "node:test";
import { createClientId } from "./helpers.js";

test("createClientId returns distinct non-empty identifiers", () => {
    const firstId = createClientId();
    const secondId = createClientId();

    assert.equal(typeof firstId, "string");
    assert.notEqual(firstId, "");
    assert.notEqual(firstId, secondId);
});