ALTER TABLE organization_positions
    ADD CONSTRAINT uq_organization_positions_organization_id_id
        UNIQUE (organization_id, id);

CREATE TABLE organization_position_connections (
    organization_id BIGINT NOT NULL
        REFERENCES organizations(id) ON DELETE CASCADE,
    source_position_id UUID NOT NULL,
    target_position_id UUID NOT NULL,

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT fk_position_connection_source
        FOREIGN KEY (organization_id, source_position_id)
        REFERENCES organization_positions (organization_id, id)
        ON DELETE CASCADE,

    CONSTRAINT fk_position_connection_target
        FOREIGN KEY (organization_id, target_position_id)
        REFERENCES organization_positions (organization_id, id)
        ON DELETE CASCADE,

    CONSTRAINT chk_position_connection_not_self
        CHECK (source_position_id <> target_position_id),

    CONSTRAINT uq_organization_position_connection
        UNIQUE (organization_id, source_position_id, target_position_id)
);