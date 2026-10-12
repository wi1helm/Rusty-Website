use std::io::Cursor;

use axum::http::StatusCode;
use image::{DynamicImage, ImageReader};

use crate::storage::database::ProfileImageType;

use super::MAX_IMAGE_SIZE;


#[derive(Debug)]
pub enum ImageValidationError {
    MissingImage,
    FileTooLarge,
    InvalidContentType,
    InvalidImage,
    InvalidImageFormat,
    DimensionsTooSmall,
    DimensionsTooLarge,
    TooManyPixels,
    InvalidAspectRatio,
}

impl ImageValidationError {
    pub fn status_code(&self) -> StatusCode {
        match self {
            Self::MissingImage => StatusCode::BAD_REQUEST,
            Self::InvalidContentType => StatusCode::BAD_REQUEST,
            Self::InvalidImage => StatusCode::BAD_REQUEST,
            Self::InvalidImageFormat => StatusCode::BAD_REQUEST,

            Self::FileTooLarge => StatusCode::PAYLOAD_TOO_LARGE,

            Self::DimensionsTooSmall => StatusCode::UNPROCESSABLE_ENTITY,
            Self::DimensionsTooLarge => StatusCode::UNPROCESSABLE_ENTITY,
            Self::TooManyPixels => StatusCode::UNPROCESSABLE_ENTITY,
            Self::InvalidAspectRatio => StatusCode::UNPROCESSABLE_ENTITY,
        }
    }
}
/// An image that has successfully passed all profile image validation checks.
///
/// The image data has been validated for size, MIME type, file format,
/// dimensions, pixel count, aspect ratio, and successful decoding.
/// The original image bytes are retained for subsequent processing.
pub struct ValidatedImage {
    pub image_type: ProfileImageType,
    pub image: DynamicImage,
    pub width: u32,
    pub height: u32,
}
/// Validates uploaded profile images against the application's image policy.
///
/// Validation includes file size, declared MIME type, detected image format,
/// image dimensions, pixel count, aspect ratio, and successful decoding.
///
/// This type is responsible only for validation. Image normalization,
/// storage, and persistence are handled separately.
pub struct ImageValidator;

impl ImageValidator {
    pub fn validate(image_type: ProfileImageType, data: Vec<u8>, content_type: &str) -> Result<ValidatedImage, ImageValidationError> {
        if data.is_empty() { return Err(ImageValidationError::MissingImage); }

        if data.len() > MAX_IMAGE_SIZE { return Err(ImageValidationError::FileTooLarge); }

        let declared_content_type = Self::validate_content_type(content_type)?;

        let format = image::guess_format(&data)
            .map_err(|_| ImageValidationError::InvalidImageFormat)?;

        Self::validate_format(declared_content_type, format)?;

        let (width, height) = Self::read_dimensions(&data)?;

        Self::validate_dimensions(&image_type, width, height)?;

        let decoded = image::load_from_memory(&data)
            .map_err(|_| ImageValidationError::InvalidImage)?;

        Ok(ValidatedImage {
            image_type,
            image: decoded,
            width,
            height,
        })
    }

    fn validate_content_type(content_type: &str) -> Result<&'static str, ImageValidationError> {
        match content_type {
            "image/jpeg" => Ok("image/jpeg"),
            "image/png" => Ok("image/png"),
            "image/webp" => Ok("image/webp"),
            _ => Err(ImageValidationError::InvalidContentType),
        }
    }

    fn validate_format(content_type: &str, format: image::ImageFormat) -> Result<(), ImageValidationError> {
        let matches = match content_type {
            "image/jpeg" => format == image::ImageFormat::Jpeg,
            "image/png" => format == image::ImageFormat::Png,
            "image/webp" => format == image::ImageFormat::WebP,
            _ => false,
        };

        if !matches { return Err(ImageValidationError::InvalidImageFormat); }

        Ok(())
    }

    fn validate_dimensions(image_type: &ProfileImageType, width: u32, height: u32) -> Result<(), ImageValidationError> {
        let rules = image_type.rules();

        if width < rules.min_width || height < rules.min_height { return Err(ImageValidationError::DimensionsTooSmall); }
        if width > rules.max_width || height > rules.max_height { return Err(ImageValidationError::DimensionsTooLarge); }

        let pixels = u64::from(width) * u64::from(height);

        if pixels > rules.max_pixels { return Err(ImageValidationError::TooManyPixels); }

        let aspect_ratio = width as f64 / height as f64;

        if aspect_ratio < rules.min_aspect_ratio || aspect_ratio > rules.max_aspect_ratio { return Err(ImageValidationError::InvalidAspectRatio); }

        Ok(())
    }

    fn read_dimensions(data: &[u8]) -> Result<(u32, u32), ImageValidationError> {
        ImageReader::new(Cursor::new(data))
            .with_guessed_format()
            .map_err(|_| ImageValidationError::InvalidImageFormat)?
            .into_dimensions()
            .map_err(|_| ImageValidationError::InvalidImage)
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::*;

    fn create_image(width: u32, height: u32, format: image::ImageFormat) -> Vec<u8> {
        let image = image::DynamicImage::new_rgb8(width, height);
        let mut data = Vec::new();

        image.write_to(&mut Cursor::new(&mut data), format).unwrap();

        data
    }

    // -------------------------------------------------------------------------
    // Image rules
    // -------------------------------------------------------------------------

    #[test]
    fn logo_rules_are_expected() {
        let rules = ProfileImageType::Logo.rules();

        assert_eq!(rules.min_width, 200);
        assert_eq!(rules.min_height, 200);
        assert_eq!(rules.max_width, 5000);
        assert_eq!(rules.max_height, 5000);
        assert_eq!(rules.max_pixels, 25_000_000);
        assert_eq!(rules.min_aspect_ratio, 0.75);
        assert_eq!(rules.max_aspect_ratio, 1.33);
    }

    #[test]
    fn banner_rules_are_expected() {
        let rules = ProfileImageType::Banner.rules();

        assert_eq!(rules.min_width, 800);
        assert_eq!(rules.min_height, 200);
        assert_eq!(rules.max_width, 5000);
        assert_eq!(rules.max_height, 2500);
        assert_eq!(rules.max_pixels, 25_000_000);
        assert_eq!(rules.min_aspect_ratio, 2.0);
        assert_eq!(rules.max_aspect_ratio, 5.0);
    }

    // -------------------------------------------------------------------------
    // Dimension validation
    // -------------------------------------------------------------------------

    #[test]
    fn accepts_valid_logo_dimensions() {
        let rules = ProfileImageType::Logo.rules();

        let result = ImageValidator::validate_dimensions(
            &ProfileImageType::Logo,
            rules.min_width,
            rules.min_height,
        );

        assert!(result.is_ok());
    }

    #[test]
    fn accepts_valid_banner_dimensions() {
        let rules = ProfileImageType::Banner.rules();

        let result = ImageValidator::validate_dimensions(
            &ProfileImageType::Banner,
            rules.min_width,
            rules.min_height,
        );

        assert!(result.is_ok());
    }

    #[test]
    fn rejects_width_below_minimum() {
        let rules = ProfileImageType::Logo.rules();

        let result = ImageValidator::validate_dimensions(
            &ProfileImageType::Logo,
            rules.min_width - 1,
            rules.min_height,
        );

        assert!(matches!(
            result,
            Err(ImageValidationError::DimensionsTooSmall)
        ));
    }

    #[test]
    fn rejects_height_below_minimum() {
        let rules = ProfileImageType::Logo.rules();

        let result = ImageValidator::validate_dimensions(
            &ProfileImageType::Logo,
            rules.min_width,
            rules.min_height - 1,
        );

        assert!(matches!(
            result,
            Err(ImageValidationError::DimensionsTooSmall)
        ));
    }

    #[test]
    fn rejects_width_above_maximum() {
        let rules = ProfileImageType::Logo.rules();

        let result = ImageValidator::validate_dimensions(
            &ProfileImageType::Logo,
            rules.max_width + 1,
            rules.min_height,
        );

        assert!(matches!(
            result,
            Err(ImageValidationError::DimensionsTooLarge)
        ));
    }

    #[test]
    fn rejects_height_above_maximum() {
        let rules = ProfileImageType::Logo.rules();

        let result = ImageValidator::validate_dimensions(
            &ProfileImageType::Logo,
            rules.min_width,
            rules.max_height + 1,
        );

        assert!(matches!(
            result,
            Err(ImageValidationError::DimensionsTooLarge)
        ));
    }

    #[test]
    fn rejects_logo_that_is_too_wide() {
        let rules = ProfileImageType::Logo.rules();

        let width = 1400;
        let height = 1000;

        assert!(width as f64 / height as f64 > rules.max_aspect_ratio);

        let result = ImageValidator::validate_dimensions(
            &ProfileImageType::Logo,
            width,
            height,
        );

        assert!(matches!(
            result,
            Err(ImageValidationError::InvalidAspectRatio)
        ));
    }

    #[test]
    fn rejects_logo_that_is_too_tall() {
        let rules = ProfileImageType::Logo.rules();

        let width = 1000;
        let height = 1400;

        assert!((width as f64 / height as f64) < rules.min_aspect_ratio);

        let result = ImageValidator::validate_dimensions(
            &ProfileImageType::Logo,
            width,
            height,
        );

        assert!(matches!(
            result,
            Err(ImageValidationError::InvalidAspectRatio)
        ));
    }

    #[test]
    fn rejects_banner_that_is_too_narrow() {
        let rules = ProfileImageType::Banner.rules();

        let width = 1000;
        let height = 1000;

        assert!((width as f64 / height as f64) < rules.min_aspect_ratio);

        let result = ImageValidator::validate_dimensions(
            &ProfileImageType::Banner,
            width,
            height,
        );

        assert!(matches!(
            result,
            Err(ImageValidationError::InvalidAspectRatio)
        ));
    }

    #[test]
    fn rejects_banner_that_is_too_wide() {
        let rules = ProfileImageType::Banner.rules();

        let width = 5000;
        let height = 500;

        assert!(width as f64 / height as f64 > rules.max_aspect_ratio);

        let result = ImageValidator::validate_dimensions(
            &ProfileImageType::Banner,
            width,
            height,
        );

        assert!(matches!(
            result,
            Err(ImageValidationError::InvalidAspectRatio)
        ));
    }

    // -------------------------------------------------------------------------
    // Dimension reading
    // -------------------------------------------------------------------------

    #[test]
    fn reads_dimensions_from_valid_image() {
        let data = create_image(
            800,
            600,
            image::ImageFormat::Png,
        );

        let result = ImageValidator::read_dimensions(&data);

        assert_eq!(result.unwrap(), (800, 600));
    }

    #[test]
    fn rejects_invalid_image_data_when_reading_dimensions() {
        let data = b"not an image";

        let result = ImageValidator::read_dimensions(data);

        assert!(matches!(
            result,
            Err(ImageValidationError::InvalidImageFormat)
                | Err(ImageValidationError::InvalidImage)
        ));
    }

    // -------------------------------------------------------------------------
    // Content type and format validation
    // -------------------------------------------------------------------------

    #[test]
    fn accepts_png_with_png_content_type() {
        let data = create_image(
            800,
            200,
            image::ImageFormat::Png,
        );

        let result = ImageValidator::validate(
            ProfileImageType::Banner,
            data,
            "image/png",
        );

        assert!(result.is_ok());
    }

    #[test]
    fn accepts_jpeg_with_jpeg_content_type() {
        let data = create_image(
            800,
            400,
            image::ImageFormat::Jpeg,
        );

        let result = ImageValidator::validate(
            ProfileImageType::Banner,
            data,
            "image/jpeg",
        );

        assert!(result.is_ok());
    }

    #[test]
    fn accepts_webp_with_webp_content_type() {
        let data = create_image(
            800,
            400,
            image::ImageFormat::WebP,
        );

        let result = ImageValidator::validate(
            ProfileImageType::Banner,
            data,
            "image/webp",
        );

        assert!(result.is_ok());
    }

    #[test]
    fn rejects_unsupported_content_type() {
        let data = create_image(
            800,
            600,
            image::ImageFormat::Png,
        );

        let result = ImageValidator::validate(
            ProfileImageType::Banner,
            data,
            "image/gif",
        );

        assert!(matches!(
            result,
            Err(ImageValidationError::InvalidContentType)
        ));
    }

    #[test]
    fn rejects_mismatched_content_type() {
        let data = create_image(
            800,
            600,
            image::ImageFormat::Png,
        );

        let result = ImageValidator::validate(
            ProfileImageType::Banner,
            data,
            "image/jpeg",
        );

        assert!(matches!(
            result,
            Err(ImageValidationError::InvalidImageFormat)
        ));
    }

    // -------------------------------------------------------------------------
    // Full validation
    // -------------------------------------------------------------------------

    #[test]
    fn rejects_empty_image() {
        let result = ImageValidator::validate(
            ProfileImageType::Logo,
            Vec::new(),
            "image/png",
        );

        assert!(matches!(
            result,
            Err(ImageValidationError::MissingImage)
        ));
    }

    #[test]
    fn rejects_image_that_is_too_large() {
        let data = vec![0u8; MAX_IMAGE_SIZE + 1];

        let result = ImageValidator::validate(
            ProfileImageType::Logo,
            data,
            "image/png",
        );

        assert!(matches!(
            result,
            Err(ImageValidationError::FileTooLarge)
        ));
    }

    #[test]
    fn rejects_malformed_image() {
        let data = b"not actually an image".to_vec();

        let result = ImageValidator::validate(
            ProfileImageType::Logo,
            data,
            "image/png",
        );

        assert!(matches!(
            result,
            Err(ImageValidationError::InvalidImageFormat)
                | Err(ImageValidationError::InvalidImage)
        ));
    }

    #[test]
    fn rejects_valid_image_with_dimensions_too_small() {
        let data = create_image(
            199,
            200,
            image::ImageFormat::Png,
        );

        let result = ImageValidator::validate(
            ProfileImageType::Logo,
            data,
            "image/png",
        );

        assert!(matches!(
            result,
            Err(ImageValidationError::DimensionsTooSmall)
        ));
    }

    #[test]
    fn rejects_valid_image_with_dimensions_too_large() {
        let data = create_image(
            5001,
            5000,
            image::ImageFormat::Png,
        );

        let result = ImageValidator::validate(
            ProfileImageType::Logo,
            data,
            "image/png",
        );

        assert!(matches!(
            result,
            Err(ImageValidationError::DimensionsTooLarge)
        ));
    }

    #[test]
    fn rejects_valid_image_with_invalid_aspect_ratio() {
        let data = create_image(
            5000,
            1000,
            image::ImageFormat::Png,
        );

        let result = ImageValidator::validate(
            ProfileImageType::Logo,
            data,
            "image/png",
        );

        assert!(matches!(
            result,
            Err(ImageValidationError::InvalidAspectRatio)
        ));
    }
}