use zeta_protocol::ContentPart;
use zeta_protocol::ImageDetail;
use zeta_utils_image::ImageAnimationPolicy;
use zeta_utils_image::ImageMetadataPolicy;
use zeta_utils_image::ImageProcessingError;
use zeta_utils_image::ImageSafetyLimits;
use zeta_utils_image::PromptImageMode;
use zeta_utils_image::PromptImagePolicy;
use zeta_utils_image::load_data_url_for_prompt;

const MAX_PRODUCT_IMAGE_BYTES: usize = 16 * 1024 * 1024;
const MAX_PRODUCT_IMAGE_DIMENSION: u32 = 32_768;
const MAX_PRODUCT_IMAGE_PIXELS: u64 = 64 * 1024 * 1024;
const MAX_PRODUCT_IMAGE_DECODED_BYTES: u64 = 256 * 1024 * 1024;
const MAX_PRODUCT_IMAGE_FRAMES: u32 = 32;

pub(crate) const IMAGE_PROCESSING_ERROR_PLACEHOLDER: &str =
    "image content omitted because it could not be processed";

pub(crate) fn prepare_user_image_data_url(image_url: &str) -> Result<String, ImageProcessingError> {
    load_data_url_for_prompt(image_url, policy_for_detail(ImageDetail::Auto))
        .map(zeta_utils_image::EncodedImage::into_data_url)
}

pub(crate) fn prepare_tool_content(content: &mut [ContentPart]) {
    for part in content {
        let replacement = match part {
            ContentPart::ImageUrl { url, detail } if is_data_url(url) => {
                match load_data_url_for_prompt(url, policy_for_detail(*detail)) {
                    Ok(image) => {
                        *url = image.into_data_url();
                        None
                    }
                    Err(_) => Some(ContentPart::Text(
                        IMAGE_PROCESSING_ERROR_PLACEHOLDER.to_owned(),
                    )),
                }
            }
            ContentPart::Text(_) | ContentPart::ImageUrl { .. } => None,
        };
        if let Some(replacement) = replacement {
            *part = replacement;
        }
    }
}

fn policy_for_detail(detail: ImageDetail) -> PromptImagePolicy {
    PromptImagePolicy {
        mode: match detail {
            ImageDetail::Original => PromptImageMode::Original,
            ImageDetail::Auto | ImageDetail::Low | ImageDetail::High => {
                PromptImageMode::ResizeToFit
            }
        },
        safety_limits: ImageSafetyLimits {
            max_input_bytes: MAX_PRODUCT_IMAGE_BYTES,
            max_output_bytes: MAX_PRODUCT_IMAGE_BYTES,
            max_dimension: MAX_PRODUCT_IMAGE_DIMENSION,
            max_pixels: MAX_PRODUCT_IMAGE_PIXELS,
            max_decoded_bytes: MAX_PRODUCT_IMAGE_DECODED_BYTES,
            max_frames: MAX_PRODUCT_IMAGE_FRAMES,
        },
        metadata_policy: ImageMetadataPolicy::PreserveColorAndOrientation,
        animation_policy: ImageAnimationPolicy::FirstFrame,
    }
}

fn is_data_url(url: &str) -> bool {
    url.get(.."data:".len())
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case("data:"))
}

#[cfg(test)]
#[path = "image_preparation_tests.rs"]
mod tests;
