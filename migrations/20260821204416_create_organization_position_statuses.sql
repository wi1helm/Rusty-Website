CREATE TABLE organization_position_statuses (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    position_id UUID NOT NULL REFERENCES organization_positions(id),

    status TEXT NOT NULL,
    information JSONB NOT NULL DEFAULT '{}'::jsonb,

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT uq_position_status
        UNIQUE (position_id, status),

    CONSTRAINT uq_position_status_position_id_id
        UNIQUE (position_id, id),

    CONSTRAINT chk_position_status
        CHECK (status IN ('OPEN', 'CLOSED', 'FILLED', 'ARCHIVED')),

    CONSTRAINT chk_position_status_information
        CHECK (jsonb_typeof(information) = 'object')
);