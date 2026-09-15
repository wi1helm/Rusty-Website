CREATE TABLE profile_images (
    id BIGSERIAL PRIMARY KEY,

    profile_id BIGINT NOT NULL
        REFERENCES profiles(id)
            ON DELETE CASCADE,

    image_type TEXT NOT NULL,

    storage_key TEXT NOT NULL,
    content_type TEXT NOT NULL,

    width INTEGER NOT NULL,
    height INTEGER NOT NULL,
    file_size BIGINT NOT NULL,

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    UNIQUE (profile_id, image_type),

    CHECK (image_type IN ('logo', 'banner')),
    CHECK (width > 0),
    CHECK (height > 0),
    CHECK (file_size > 0)
);