CREATE TABLE organization_members (
    id BIGSERIAL PRIMARY KEY,

    organization_id BIGINT NOT NULL
        REFERENCES organizations(id)
            ON DELETE CASCADE,

    email TEXT NOT NULL UNIQUE,
    password_hash TEXT NOT NULL,
    role TEXT NOT NULL,

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT organization_members_role_check
        CHECK (role IN ('owner', 'admin','member'))
);
