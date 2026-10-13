/* =========================================================
   shared/link-icons.js  (SHARED: public + protected)

   Picks an icon for an external link:
   known services get an inline icon, everything else gets
   the site's favicon, with a generic link icon as fallback.
========================================================= */

import { createElement, createSvgIcon, icons } from "./helpers.js";


const hostMatches = (hostname, domain) =>
    hostname === domain || hostname.endsWith(`.${domain}`);


export function getLinkIconType(url) {
    let hostname;

    try {
        hostname = new URL(url).hostname.toLowerCase();
    } catch {
        return "link";
    }

    if (hostMatches(hostname, "linkedin.com")) return "linkedin";
    if (hostMatches(hostname, "github.com")) return "github";
    if (hostMatches(hostname, "instagram.com")) return "instagram";
    if (hostMatches(hostname, "facebook.com") || hostname === "fb.com") return "facebook";
    if (hostMatches(hostname, "youtube.com") || hostname === "youtu.be") return "youtube";
    if (hostMatches(hostname, "x.com") || hostMatches(hostname, "twitter.com")) return "x";
    /*TODO check if this is bad downloadig favicon etc. could make own website with favicon etc bad stuff maby*/
    return "favicon";
}


export function getFaviconUrl(url) {
    const parsed = new URL(url);

    const faviconUrl = new URL("https://www.google.com/s2/favicons");
    faviconUrl.searchParams.set("domain", parsed.hostname);
    faviconUrl.searchParams.set("sz", "64");

    return faviconUrl.href;
}


const svgIcons = {
    github: {
        size: 19,
        paths: [
            "M9 19c-5 1.5-5-2.5-7-3",
            "M16 22v-3.87a3.37 3.37 0 0 0-.94-2.61c3.14-.35 6.44-1.54 6.44-7A5.44 5.44 0 0 0 20 4.77 5.07 5.07 0 0 0 19.91 1S18.73.65 16 2.48a13.38 13.38 0 0 0-7 0C6.27.65 5.09 1 5.09 1A5.07 5.07 0 0 0 5 4.77a5.44 5.44 0 0 0-1.5 3.78c0 5.42 3.3 6.61 6.44 7A3.37 3.37 0 0 0 9 18.13V22"
        ]
    },
    instagram: {
        size: 19,
        paths: [
            "M7 2h10a5 5 0 0 1 5 5v10a5 5 0 0 1-5 5H7a5 5 0 0 1-5-5V7a5 5 0 0 1 5-5Z",
            "M16 11.37a4 4 0 1 1-2.37-3.37A4 4 0 0 1 16 11.37Z",
            "M17.5 6.5h.01"
        ]
    },
    youtube: {
        size: 19,
        paths: [
            "M22 12s0-3.5-.45-5.18a2.5 2.5 0 0 0-1.77-1.77C18.1 4.6 12 4.6 12 4.6s-6.1 0-7.78.45a2.5 2.5 0 0 0-1.77 1.77C2 8.5 2 12 2 12s0 3.5.45 5.18a2.5 2.5 0 0 0 1.77 1.77c1.68.45 7.78.45 7.78.45s6.1 0 7.78-.45a2.5 2.5 0 0 0 1.77-1.77C22 15.5 22 12 22 12Z",
            "m10 15 5-3-5-3z"
        ]
    },
    x: {
        size: 16,
        paths: [
            "M18 3h3l-7.5 8.5L22 21h-6l-4.7-6.2L5.8 21H2.7l8-9.2L2 3h6.2l4.2 5.7L18 3Z"
        ]
    }
};

const textIcons = {
    linkedin: { text: "in", fontSize: "15px" },
    facebook: { text: "f", fontSize: "20px" }
};


/**
 * Fills `container` with the right icon for `url`.
 * Safe to call on server-rendered containers: existing
 * (fallback) children are replaced.
 */
export function renderLinkIcon(container, url) {
    const iconType = getLinkIconType(url);

    container.classList.add("profile-link-icon");
    container.replaceChildren();

    if (iconType === "favicon") {
        try {
            const image = createElement("img");

            image.alt = "";
            image.loading = "lazy";
            image.referrerPolicy = "no-referrer";
            image.src = getFaviconUrl(url);

            image.addEventListener("error", () => {
                container.replaceChildren(createSvgIcon(icons.link, 18));
            }, { once: true });

            container.appendChild(image);
            return;
        } catch {
            // Invalid URL: fall through to the generic icon.
        }
    }

    if (textIcons[iconType]) {
        const { text, fontSize } = textIcons[iconType];
        const icon = createElement("span", null, text);

        icon.style.fontWeight = "800";
        icon.style.fontSize = fontSize;

        container.appendChild(icon);
        return;
    }

    if (svgIcons[iconType]) {
        const { paths, size } = svgIcons[iconType];
        container.appendChild(createSvgIcon(paths, size));
        return;
    }

    container.appendChild(createSvgIcon(icons.link, 18));
}