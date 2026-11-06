CREATE TABLE organizations (
       id BIGSERIAL PRIMARY KEY,
       name TEXT NOT NULL,
       slug TEXT NOT NULL UNIQUE,
       created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
