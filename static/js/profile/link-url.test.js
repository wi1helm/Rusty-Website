import assert from "node:assert/strict";
import test from "node:test";
import { normalizeLinkUrl } from "./link-url.js";

test("normalizes valid HTTPS URLs", () => {
    assert.equal(
        normalizeLinkUrl("  HTTPS://EXAMPLE.COM:443/a b  "),
        "https://example.com/a%20b"
    );
});

test("rejects an empty URL", () => {
    assert.throws(() => normalizeLinkUrl("  "), {
        message: "Please enter a URL."
    });
});

test("rejects URLs without an explicit scheme", () => {
    assert.throws(() => normalizeLinkUrl("example.com"), {
        message: "Please enter a valid URL, including https://."
    });
});

test("rejects malformed URLs", () => {
    assert.throws(() => normalizeLinkUrl("https://"), {
        message: "Please enter a valid URL, including https://."
    });
});

test("rejects non-HTTPS protocols", () => {
    for (const url of ["http://example.com", "javascript:alert(1)"]) {
        assert.throws(() => normalizeLinkUrl(url), {
            message: "Only HTTPS links are supported."
        });
    }
});