CREATE TABLE organization_profile_images (
     id BIGSERIAL PRIMARY KEY,

     organization_id BIGINT NOT NULL
         REFERENCES organizations(id)
             ON DELETE CASCADE,

     image_type TEXT NOT NULL,

     storage_key TEXT NOT NULL,
     content_type TEXT NOT NULL,

     width INTEGER NOT NULL,
     height INTEGER NOT NULL,
     file_size BIGINT NOT NULL,

     created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
     updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

     CONSTRAINT organization_profile_images_type_check
         CHECK (image_type IN ('logo', 'banner')),

     CONSTRAINT organization_profile_images_dimensions_check
         CHECK (width > 0 AND height > 0),

     CONSTRAINT organization_profile_images_file_size_check
         CHECK (file_size > 0),

     CONSTRAINT organization_profile_images_unique_type
         UNIQUE (organization_id, image_type)
);