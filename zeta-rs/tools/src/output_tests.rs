use super::{ToolContent, ToolOutput, ToolOutputStatus};

#[test]
fn error_output_preserves_model_visible_content() {
    let output = ToolOutput::error(vec![ToolContent::Text("request denied".to_owned())]);

    assert_eq!(output.status(), ToolOutputStatus::Error);
    assert_eq!(
        output.content(),
        &[ToolContent::Text("request denied".to_owned())]
    );
}

#[test]
fn final_output_gate_sanitizes_every_image() {
    let mut output = ToolOutput::success(vec![ToolContent::Image {
        url: "data:image/png;base64,AA==".into(),
        detail: crate::ImageDetail::Original,
    }]);

    let decisions = output.sanitize_image_detail(
        crate::ImageDetailCapabilities { original: false },
        crate::ImageSourceDetailPolicy::Preserve,
    );

    assert_eq!(decisions.len(), 1);
    assert!(matches!(
        output.content(),
        [ToolContent::Image {
            detail: crate::ImageDetail::Auto,
            ..
        }]
    ));
}
