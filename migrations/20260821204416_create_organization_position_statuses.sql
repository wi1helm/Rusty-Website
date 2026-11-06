-- Add migration script here
CREATE TABLE organization_position_statuses (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    position_id UUID NOT NULL REFERENCES organization_positions(id),

    status TEXT NOT NULL,
    information TEXT,
    application_url TEXT,

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

ALTER TABLE organization_position_statuses
    ADD CONSTRAINT uq_position_status_position_id_id
        UNIQUE (position_id, id);

ALTER TABLE organization_positions
    ADD CONSTRAINT fk_position_current_status
        FOREIGN KEY (id, current_status_id)
            REFERENCES organization_position_statuses(position_id, id);

