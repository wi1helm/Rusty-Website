CREATE TABLE organizations (
       id BIGSERIAL PRIMARY KEY,
       name TEXT NOT NULL,
       created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
