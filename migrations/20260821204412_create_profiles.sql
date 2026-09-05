-- Add migration script here
CREATE TABLE profiles (
      id BIGSERIAL PRIMARY KEY,

      organization_id BIGINT NOT NULL UNIQUE
      REFERENCES organizations(id)
      ON DELETE CASCADE,

      current_version_id BIGINT,
      published_version_id BIGINT,

      created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
      updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE profile_versions (
      id BIGSERIAL PRIMARY KEY,

      profile_id BIGINT NOT NULL
      REFERENCES profiles(id)
      ON DELETE CASCADE,

      version INTEGER NOT NULL,

      display_name TEXT NOT NULL,
      about TEXT NOT NULL DEFAULT '',

      created_by_member_id BIGINT
      REFERENCES members(id),

      created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

      UNIQUE (profile_id, version)
);

ALTER TABLE profiles
    ADD CONSTRAINT fk_profiles_current_version
        FOREIGN KEY (current_version_id)
        REFERENCES profile_versions(id);

ALTER TABLE profiles
    ADD CONSTRAINT fk_profiles_published_version
        FOREIGN KEY (published_version_id)
        REFERENCES profile_versions(id);