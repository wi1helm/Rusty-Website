-- Add migration script here
CREATE TABLE profile_version_links (
       id BIGSERIAL PRIMARY KEY,

       profile_version_id BIGINT NOT NULL
           REFERENCES profile_versions(id)
               ON DELETE CASCADE,

       label TEXT NOT NULL,
       url TEXT NOT NULL,

       position INTEGER NOT NULL DEFAULT 0,

       created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

       CONSTRAINT profile_version_links_label_not_empty
           CHECK (LENGTH(TRIM(label)) > 0),

       CONSTRAINT profile_version_links_url_not_empty
           CHECK (LENGTH(TRIM(url)) > 0),

       CONSTRAINT profile_version_links_position_non_negative
           CHECK (position >= 0),

       CONSTRAINT profile_version_links_unique_position
           UNIQUE (profile_version_id, position)
);

CREATE INDEX idx_profile_version_links_version
    ON profile_version_links(profile_version_id);