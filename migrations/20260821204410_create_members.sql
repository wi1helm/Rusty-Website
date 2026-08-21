CREATE TABLE members (
     id BIGSERIAL PRIMARY KEY,
     organization_id BIGINT NOT NULL,
     email TEXT NOT NULL UNIQUE,
     password_hash TEXT NOT NULL,
     role TEXT NOT NULL,
     created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

     FOREIGN KEY (organization_id)
         REFERENCES organizations(id)
         ON DELETE CASCADE
);
