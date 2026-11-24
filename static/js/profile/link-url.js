export function normalizeLinkUrl(value) {
    const raw = value.trim();

    if (!raw) {
        throw new Error("Please enter a URL.");
    }

    let parsed;

    try {
        parsed = new URL(raw);
    } catch {
        throw new Error("Please enter a valid URL, including https://.");
    }

    if (parsed.protocol !== "https:") {
        throw new Error("Only HTTPS links are supported.");
    }

    if (!parsed.hostname) {
        throw new Error("Please enter a valid website address.");
    }

    return parsed.href;
}