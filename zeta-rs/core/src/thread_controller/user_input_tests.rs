use super::*;

#[test]
fn accepts_supported_image_data_and_remote_urls() {
    let png = format!(
        "data:image/png;base64,{}",
        STANDARD.encode(b"\x89PNG\r\n\x1a\npayload")
    );

    assert!(validate_image_url(&png).is_ok());
    assert!(validate_image_url("https://example.test/image.png").is_ok());
}

#[test]
fn rejects_mismatched_image_mime_type() {
    let jpeg_with_png_data = format!(
        "data:image/jpeg;base64,{}",
        STANDARD.encode(b"\x89PNG\r\n\x1a\npayload")
    );

    assert!(matches!(
        validate_image_url(&jpeg_with_png_data),
        Err(CoreError::InvalidInput(_))
    ));
}

#[test]
fn rejects_local_paths_at_the_core_boundary() {
    assert!(matches!(
        validate(&[UserInput::LocalImage {
            path: "/tmp/image.png".into(),
        }]),
        Err(CoreError::InvalidInput(_))
    ));
}
