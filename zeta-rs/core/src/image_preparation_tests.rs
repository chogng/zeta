use super::*;
use zeta_protocol::ContentPart;
use zeta_protocol::ImageDetail;

#[test]
fn prepares_valid_user_data_urls() {
    let source = crate::test_image::one_pixel_png_data_url();
    let prepared = prepare_user_image_data_url(&source).expect("prepare image");
    assert_eq!(prepared, source);
}

#[test]
fn replaces_invalid_tool_images_without_dropping_other_content() {
    let mut content = vec![
        ContentPart::Text("before".into()),
        ContentPart::ImageUrl {
            url: "data:image/png;base64,AA==".into(),
            detail: ImageDetail::High,
        },
        ContentPart::Text("after".into()),
    ];

    prepare_tool_content(&mut content);

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
fn leaves_remote_tool_images_for_the_provider_transport() {
    let mut content = vec![ContentPart::ImageUrl {
        url: "https://example.test/image.png".into(),
        detail: ImageDetail::Auto,
    }];

    prepare_tool_content(&mut content);

    assert!(matches!(
        &content[0],
        ContentPart::ImageUrl { url, .. } if url == "https://example.test/image.png"
    ));
}
