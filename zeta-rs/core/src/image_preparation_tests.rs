use std::sync::Arc;

use super::*;
use zeta_attachments::ImageAttachments;
use zeta_protocol::ContentPart;
use zeta_protocol::ImageDetail;

#[test]
fn replaces_valid_data_urls_with_durable_references() {
    let attachments = Arc::new(ImageAttachments::in_memory());
    let mut content = vec![ContentPart::ImageUrl {
        url: crate::test_image::one_pixel_png_data_url(),
        detail: ImageDetail::Auto,
    }];

    prepare_tool_content(&mut content, &attachments);

    assert!(matches!(content[0], ContentPart::ImageAttachment { .. }));
}

#[test]
fn replaces_invalid_tool_images_without_dropping_other_content() {
    let attachments = Arc::new(ImageAttachments::in_memory());
    let mut content = vec![
        ContentPart::Text("before".into()),
        ContentPart::ImageUrl {
            url: "data:image/png;base64,AA==".into(),
            detail: ImageDetail::High,
        },
        ContentPart::Text("after".into()),
    ];

    prepare_tool_content(&mut content, &attachments);

    assert_eq!(
        content,
        vec![
            ContentPart::Text("before".into()),
            ContentPart::Text(IMAGE_PROCESSING_ERROR_PLACEHOLDER.into()),
            ContentPart::Text("after".into()),
        ]
    );
}

#[test]
fn omits_remote_tool_images_when_no_safe_fetcher_is_installed() {
    let attachments = Arc::new(ImageAttachments::in_memory());
    let mut content = vec![ContentPart::ImageUrl {
        url: "https://example.test/image.png".into(),
        detail: ImageDetail::Auto,
    }];

    prepare_tool_content(&mut content, &attachments);

    assert_eq!(
        content,
        vec![ContentPart::Text(IMAGE_PROCESSING_ERROR_PLACEHOLDER.into())]
    );
}
