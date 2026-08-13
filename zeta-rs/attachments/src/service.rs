use std::path::Path;
use std::sync::Arc;

use zeta_protocol::ContentDigest;
use zeta_protocol::ImageAttachmentRef;
use zeta_protocol::ImageDetail;
use zeta_protocol::ImageMediaType;
use zeta_utils_image::EncodedImage;
use zeta_utils_image::ImageAnimationPolicy;
use zeta_utils_image::ImageMetadataPolicy;
use zeta_utils_image::ImageSafetyLimits;
use zeta_utils_image::PromptImageMode;
use zeta_utils_image::PromptImagePolicy;
use zeta_utils_image::SupportedImageFormat;
use zeta_utils_image::data_url_from_bytes;
use zeta_utils_image::detect_image_format;
use zeta_utils_image::load_data_url_for_prompt;
use zeta_utils_image::load_for_prompt_bytes;

use crate::AttachmentError;
use crate::ImageAttachmentStore;
use crate::MAX_IMAGE_ATTACHMENT_BYTES;
use crate::MemoryImageAttachmentStore;
use crate::RemoteImageFetcher;

const MAX_PRODUCT_IMAGE_DIMENSION: u32 = 32_768;
const MAX_PRODUCT_IMAGE_PIXELS: u64 = 64 * 1024 * 1024;
const MAX_PRODUCT_IMAGE_DECODED_BYTES: u64 = 256 * 1024 * 1024;
const MAX_PRODUCT_IMAGE_FRAMES: u32 = 32;

/// Canonical service that validates untrusted images before they enter durable storage.
pub struct ImageAttachments {
    store: Arc<dyn ImageAttachmentStore>,
    remote: Option<Arc<dyn RemoteImageFetcher>>,
}

impl ImageAttachments {
    pub fn new(store: Arc<dyn ImageAttachmentStore>) -> Self {
        Self {
            store,
            remote: None,
        }
    }

    pub fn in_memory() -> Self {
        Self::new(Arc::new(MemoryImageAttachmentStore::default()))
    }

    pub fn with_remote_fetcher(mut self, remote: Arc<dyn RemoteImageFetcher>) -> Self {
        self.remote = Some(remote);
        self
    }

    pub fn import_data_url(
        &self,
        data_url: &str,
        detail: ImageDetail,
    ) -> Result<ImageAttachmentRef, AttachmentError> {
        let image = load_data_url_for_prompt(data_url, policy_for_detail(detail))
            .map_err(|error| AttachmentError::InvalidImage(error.to_string()))?;
        self.store.put(&image)
    }

    pub fn import_bytes(
        &self,
        bytes: Vec<u8>,
        detail: ImageDetail,
    ) -> Result<ImageAttachmentRef, AttachmentError> {
        let image = load_for_prompt_bytes(
            Path::new("<attachment-upload>"),
            bytes,
            policy_for_detail(detail),
        )
        .map_err(|error| AttachmentError::InvalidImage(error.to_string()))?;
        self.store.put(&image)
    }

    pub fn import_remote_url(
        &self,
        url: &str,
        detail: ImageDetail,
    ) -> Result<ImageAttachmentRef, AttachmentError> {
        let bytes = self
            .remote
            .as_ref()
            .ok_or(AttachmentError::RemoteUnavailable)?
            .fetch(url)?;
        self.import_bytes(bytes, detail)
    }

    pub fn verify(&self, reference: &ImageAttachmentRef) -> Result<(), AttachmentError> {
        self.read_verified(reference).map(|_| ())
    }

    pub fn materialize_data_url(
        &self,
        reference: &ImageAttachmentRef,
    ) -> Result<String, AttachmentError> {
        let bytes = self.read_verified(reference)?;
        Ok(data_url_from_bytes(reference.media_type.mime_type(), &bytes))
    }

    fn read_verified(
        &self,
        reference: &ImageAttachmentRef,
    ) -> Result<Arc<[u8]>, AttachmentError> {
        validate_reference_shape(reference)?;
        self.store.read(reference)
    }
}

pub fn image_media_type(bytes: &[u8]) -> Option<ImageMediaType> {
    detect_image_format(bytes).map(media_type_for_format)
}

pub(crate) fn reference_for_image(image: &EncodedImage) -> Result<ImageAttachmentRef, AttachmentError> {
    let encoded_bytes = u64::try_from(image.bytes.len()).map_err(|_| AttachmentError::TooLarge)?;
    if image.bytes.is_empty() || image.bytes.len() > MAX_IMAGE_ATTACHMENT_BYTES {
        return Err(AttachmentError::TooLarge);
    }
    if image.width == 0 || image.height == 0 {
        return Err(AttachmentError::Corrupt);
    }
    let media_type = media_type_for_mime(image.mime).ok_or(AttachmentError::Corrupt)?;
    Ok(ImageAttachmentRef {
        content_digest: ContentDigest::sha256(&image.bytes),
        media_type,
        encoded_bytes,
        width: image.width,
        height: image.height,
    })
}

pub(crate) fn verify_reference_bytes(
    reference: &ImageAttachmentRef,
    bytes: &[u8],
) -> Result<(), AttachmentError> {
    validate_reference_shape(reference)?;
    if usize::try_from(reference.encoded_bytes).ok() != Some(bytes.len())
        || ContentDigest::sha256(bytes) != reference.content_digest
        || image_media_type(bytes) != Some(reference.media_type)
    {
        return Err(AttachmentError::Corrupt);
    }
    let image = load_for_prompt_bytes(
        Path::new("<stored-attachment>"),
        bytes.to_vec(),
        policy_for_detail(ImageDetail::Original),
    )
    .map_err(|_| AttachmentError::Corrupt)?;
    if image.width != reference.width || image.height != reference.height {
        return Err(AttachmentError::Corrupt);
    }
    Ok(())
}

fn validate_reference_shape(reference: &ImageAttachmentRef) -> Result<(), AttachmentError> {
    if reference.encoded_bytes == 0
        || reference.encoded_bytes > MAX_IMAGE_ATTACHMENT_BYTES as u64
        || reference.width == 0
        || reference.height == 0
        || reference.width > MAX_PRODUCT_IMAGE_DIMENSION
        || reference.height > MAX_PRODUCT_IMAGE_DIMENSION
    {
        return Err(AttachmentError::Corrupt);
    }
    Ok(())
}

fn media_type_for_format(format: SupportedImageFormat) -> ImageMediaType {
    match format {
        SupportedImageFormat::Png => ImageMediaType::Png,
        SupportedImageFormat::Jpeg => ImageMediaType::Jpeg,
        SupportedImageFormat::Gif => ImageMediaType::Gif,
        SupportedImageFormat::WebP => ImageMediaType::WebP,
    }
}

fn media_type_for_mime(mime: &str) -> Option<ImageMediaType> {
    match mime {
        "image/png" => Some(ImageMediaType::Png),
        "image/jpeg" => Some(ImageMediaType::Jpeg),
        "image/gif" => Some(ImageMediaType::Gif),
        "image/webp" => Some(ImageMediaType::WebP),
        _ => None,
    }
}

fn policy_for_detail(detail: ImageDetail) -> PromptImagePolicy {
    PromptImagePolicy {
        mode: match detail {
            ImageDetail::Original => PromptImageMode::Original,
            ImageDetail::Auto | ImageDetail::Low | ImageDetail::High => PromptImageMode::ResizeToFit,
        },
        safety_limits: ImageSafetyLimits {
            max_input_bytes: MAX_IMAGE_ATTACHMENT_BYTES,
            max_output_bytes: MAX_IMAGE_ATTACHMENT_BYTES,
            max_dimension: MAX_PRODUCT_IMAGE_DIMENSION,
            max_pixels: MAX_PRODUCT_IMAGE_PIXELS,
            max_decoded_bytes: MAX_PRODUCT_IMAGE_DECODED_BYTES,
            max_frames: MAX_PRODUCT_IMAGE_FRAMES,
        },
        metadata_policy: ImageMetadataPolicy::PreserveColorAndOrientation,
        animation_policy: ImageAnimationPolicy::FirstFrame,
    }
}
