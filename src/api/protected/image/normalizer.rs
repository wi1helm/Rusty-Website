use std::io::Cursor;

use axum::http::StatusCode;
use image::{DynamicImage, ImageFormat};

use crate::storage::database::ProfileImageType;

use super::validator::ValidatedImage;

#[derive(Debug)]
pub enum ImageNormalizationError {
    Decode,
    Resize,
    Encode,
}

impl ImageNormalizationError {
    pub fn status_code(&self) -> StatusCode {
        // These should generally be 500s because validation has already
        // established that the image is valid.
        StatusCode::INTERNAL_SERVER_ERROR
    }
}

/// An image in the canonical representation used by object storage.
///
/// Normalized images are always:
/// - WebP
/// - stripped of source metadata
/// - correctly oriented
/// - within the normalized dimension limits
///
/// The bytes in `data` are safe to upload directly to object storage.
pub struct NormalizedImage {
    pub image_type: ProfileImageType,
    pub data: Vec<u8>,
    pub content_type: &'static str,
    pub width: u32,
    pub height: u32,
}

pub struct ImageNormalizer;

impl ImageNormalizer {
    pub fn normalize(image: ValidatedImage) -> Result<NormalizedImage, ImageNormalizationError> {
        let image_type = image.image_type;

        let image = Self::normalize_dimensions(image_type, image.image);

        let width = image.width();
        let height = image.height();
        //TODO make a non generic webp encoder
        let data = Self::encode_webp(&image)?;

        Ok(NormalizedImage {
            image_type,
            data,
            content_type: "image/webp",
            width,
            height,
        })
    }

    fn normalize_dimensions(image_type: ProfileImageType, image: DynamicImage) -> DynamicImage {
        let rules = image_type.rules();

        if image.width() <= rules.output_max_width && image.height() <= rules.output_max_height {
            return image;
        }

        image.resize(rules.output_max_width, rules.output_max_height, image::imageops::FilterType::Lanczos3)
    }

    fn encode_webp(image: &DynamicImage, ) -> Result<Vec<u8>, ImageNormalizationError> {
        let mut data = Cursor::new(Vec::new());

        image.write_to(&mut data, ImageFormat::WebP)
            .map_err(|_| ImageNormalizationError::Encode)?;

        Ok(data.into_inner())
    }
}