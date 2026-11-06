use std::io::Cursor;

use image::ImageReader;

use crate::config::{DEFAULT_BANNER_KEY, DEFAULT_LOGO_KEY};
use crate::storage::database::{NewProfileImage, ProfileImageType};

fn create_default_profile_image(
    organization_id: i64,
    image_type: ProfileImageType,
    storage_key: &str,
    data: &[u8],
) -> Result<NewProfileImage, String> {
    let reader = ImageReader::new(Cursor::new(data))
        .with_guessed_format()
        .map_err(|error| error.to_string())?;

    let (width, height) = reader
        .into_dimensions()
        .map_err(|error| error.to_string())?;

    let width = i32::try_from(width)
        .map_err(|error| error.to_string())?;

    let height = i32::try_from(height)
        .map_err(|error| error.to_string())?;

    if width <= 0 || height <= 0 {
        return Err("Image dimensions must be positive".to_string());
    }

    let file_size = i64::try_from(data.len())
        .map_err(|error| error.to_string())?;

    Ok(NewProfileImage {
        organization_id,
        image_type,
        storage_key: storage_key.to_string(),
        content_type: "image/webp".to_string(),
        width,
        height,
        file_size,
    })
}

pub fn default_logo(
    organization_id: i64,
) -> Result<NewProfileImage, String> {
    create_default_profile_image(
        organization_id,
        ProfileImageType::Logo,
        DEFAULT_LOGO_KEY,
        include_bytes!("../../../static/defaults/default_logo.webp"),
    )
}

pub fn default_banner(organization_id: i64, ) -> Result<NewProfileImage, String> {
    create_default_profile_image(
        organization_id,
        ProfileImageType::Banner,
        DEFAULT_BANNER_KEY,
        include_bytes!("../../../static/defaults/default_banner.webp"),
    )
}