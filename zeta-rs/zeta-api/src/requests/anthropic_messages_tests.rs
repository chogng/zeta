use super::*;
use crate::ImageDetail;

#[test]
fn converts_remote_image_to_anthropic_url_source() {
    let converted = convert_content(&ContentPart::ImageUrl {
        url: "https://example.com/image.png".into(),
        detail: ImageDetail::Auto,
    })
    .unwrap();

    assert_eq!(
        converted,
        json!({
            "type": "image",
            "source": {
                "type": "url",
                "url": "https://example.com/image.png",
            },
        })
    );
}

#[test]
fn converts_data_url_to_anthropic_base64_source() {
    let converted = convert_content(&ContentPart::ImageUrl {
        url: "data:image/png;base64,iVBORw0KGgo=".into(),
        detail: ImageDetail::Auto,
    })
    .unwrap();

    assert_eq!(
        converted,
        json!({
            "type": "image",
            "source": {
                "type": "base64",
                "media_type": "image/png",
                "data": "iVBORw0KGgo=",
            },
        })
    );
}

#[test]
fn rejects_unsupported_image_data_url() {
    let result = convert_content(&ContentPart::ImageUrl {
        url: "data:image/svg+xml;base64,PHN2Zz4=".into(),
        detail: ImageDetail::Auto,
    });

    assert!(matches!(result, Err(ApiError::InvalidRequest(_))));
}
