CREATE TABLE organization_profiles (
    id BIGSERIAL PRIMARY KEY,

    organization_id BIGINT NOT NULL UNIQUE
       REFERENCES organizations(id)
           ON DELETE CASCADE,

    display_name TEXT NOT NULL,
    about TEXT NOT NULL DEFAULT '',

    updated_by_member_id BIGINT NOT NULL
       REFERENCES organization_members(id),

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);