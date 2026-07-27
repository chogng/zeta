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
