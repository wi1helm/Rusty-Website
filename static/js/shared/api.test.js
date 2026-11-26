import assert from "node:assert/strict";
import test from "node:test";
import { ApiError, apiFetch, apiPostJson } from "./api.js";

test("apiFetch returns parsed JSON for successful responses", async () => {
    globalThis.fetch = async () => new Response(JSON.stringify({ redirectTo: "/dashboard" }), {
        status: 201,
        headers: { "content-type": "application/json" }
    });

    const { response, data } = await apiFetch("/api/organizations/create", { method: "POST" });

    assert.equal(response.status, 201);
    assert.deepEqual(data, { redirectTo: "/dashboard" });
});

test("apiFetch exposes the structured API error message and code", async () => {
    globalThis.fetch = async () => new Response(JSON.stringify({
        error: { code: "invalid-email", message: "Please enter a valid email address." }
    }), {
        status: 400,
        headers: { "content-type": "application/json" }
    });

    await assert.rejects(apiFetch("/api/organizations/create", { method: "POST" }), error => {
        assert.ok(error instanceof ApiError);
        assert.equal(error.status, 400);
        assert.equal(error.code, "invalid-email");
        assert.equal(error.message, "Please enter a valid email address.");
        return true;
    });
});

test("apiFetch handles empty successful responses", async () => {
    globalThis.fetch = async () => new Response(null, { status: 204 });

    const { response, data } = await apiFetch("/api/profile/image", { method: "POST" });

    assert.equal(response.status, 204);
    assert.equal(data, null);
});

test("apiPostJson sends the payload as JSON", async () => {
    let request;
    globalThis.fetch = async (url, options) => {
        request = { url, ...options };
        return new Response(JSON.stringify({ redirectTo: "/dashboard" }), {
            status: 200,
            headers: { "content-type": "application/json" }
        });
    };

    await apiPostJson("/api/auth/login", { email: "member@example.com", password: "secret" });

    assert.equal(request.url, "/api/auth/login");
    assert.equal(request.method, "POST");
    assert.equal(request.headers["Content-Type"], "application/json");
    assert.deepEqual(JSON.parse(request.body), { email: "member@example.com", password: "secret" });
    assert.equal(request.credentials, "same-origin");
});