CREATE TABLE organization_profile_links (
    id BIGSERIAL PRIMARY KEY,

    organization_id BIGINT NOT NULL
        REFERENCES organizations(id)
            ON DELETE CASCADE,

    label TEXT NOT NULL,
    url TEXT NOT NULL,

    position INTEGER NOT NULL DEFAULT 0,

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT organization_profile_links_label_not_empty
        CHECK (LENGTH(TRIM(label)) > 0),

    CONSTRAINT organization_profile_links_url_not_empty
        CHECK (LENGTH(TRIM(url)) > 0),

    CONSTRAINT organization_profile_links_position_non_negative
        CHECK (position >= 0),

    CONSTRAINT organization_profile_links_unique_position
        UNIQUE (organization_id, position)
);