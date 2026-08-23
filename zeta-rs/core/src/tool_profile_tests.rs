use super::snapshot_tool_profile;
use super::validate_tool_profile_definitions;
use serde_json::json;
use zeta_protocol::ToolDefinition;
use zeta_protocol::ToolName;

fn definition(name: &str, description: &str) -> ToolDefinition {
    ToolDefinition {
        name: ToolName::new(name).unwrap(),
        description: description.into(),
        parameters: json!({
            "type": "object",
            "properties": {},
            "required": [],
            "additionalProperties": false
        }),
        strict: true,
    }
}

#[test]
fn profile_digest_is_stable_and_order_sensitive() {
    let definitions = vec![definition("read_file", "Read"), definition("edit", "Edit")];
    let first = snapshot_tool_profile(&definitions).unwrap();
    let second = snapshot_tool_profile(&definitions).unwrap();
    assert_eq!(first, second);
    assert_eq!(
        first.definition_digest,
        "sha256:25cfb1c83f13d99c467071270a386191fbe6697f16a76a1f7f0d631a2340ed77"
    );
    assert!(validate_tool_profile_definitions(&first, &definitions).is_ok());

    let reversed = definitions.into_iter().rev().collect::<Vec<_>>();
    assert!(validate_tool_profile_definitions(&first, &reversed).is_err());
}
