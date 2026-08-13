use std::sync::Arc;

use zeta_attachments::ImageAttachments;
use zeta_protocol::ContentPart;

pub(crate) const IMAGE_PROCESSING_ERROR_PLACEHOLDER: &str =
    "image content omitted because it could not be processed";

pub(crate) fn prepare_tool_content(
    content: &mut [ContentPart],
    attachments: &Arc<ImageAttachments>,
) {
    for part in content {
        let replacement = match part {
            ContentPart::ImageUrl { url, detail } if is_data_url(url) => attachments
                .import_data_url(url, *detail)
                .map(|attachment| ContentPart::ImageAttachment {
                    attachment,
                    detail: *detail,
                })
                .unwrap_or_else(|_| {
                    ContentPart::Text(IMAGE_PROCESSING_ERROR_PLACEHOLDER.to_owned())
                }),
            ContentPart::ImageUrl { url, detail } if is_remote_image_url(url) => attachments
                .import_remote_url(url, *detail)
                .map(|attachment| ContentPart::ImageAttachment {
                    attachment,
                    detail: *detail,
                })
                .unwrap_or_else(|_| {
                    ContentPart::Text(IMAGE_PROCESSING_ERROR_PLACEHOLDER.to_owned())
                }),
            ContentPart::ImageAttachment { attachment, .. } => {
                if attachments.verify(attachment).is_ok() {
                    continue;
                }
                ContentPart::Text(IMAGE_PROCESSING_ERROR_PLACEHOLDER.to_owned())
            }
            ContentPart::Text(_) | ContentPart::ImageUrl { .. } => continue,
        };
        *part = replacement;
    }
}

fn is_data_url(url: &str) -> bool {
    url.get(.."data:".len())
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case("data:"))
}

fn is_remote_image_url(url: &str) -> bool {
    url.starts_with("https://") || url.starts_with("http://")
}

#[cfg(test)]
#[path = "image_preparation_tests.rs"]
mod tests;
