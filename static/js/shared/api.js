export class ApiError extends Error {
    constructor(message, code, status) {
        super(message);
        this.name = "ApiError";
        this.code = code;
        this.status = status;
    }
}

export async function apiFetch(url, options = {}) {
    const { headers = {}, ...requestOptions } = options;
    let response;

    try {
        response = await fetch(url, {
            credentials: "same-origin",
            ...requestOptions,
            headers: {
                Accept: "application/json",
                ...headers
            }
        });
    } catch {
        throw new ApiError("Unable to reach the server. Check your connection and try again.", "network-error", 0);
    }

    let data = null;
    if (response.headers.get("content-type")?.includes("application/json")) {
        try {
            data = await response.json();
        } catch {
            data = null;
        }
    }

    if (!response.ok) {
        const message = data?.error?.message || "Something went wrong. Please try again.";
        throw new ApiError(message, data?.error?.code || "request-failed", response.status);
    }

    return { response, data };
}

export function apiPostJson(url, payload) {
    return apiFetch(url, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(payload)
    });
}