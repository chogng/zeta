use super::catalog::{control_definitions, parse_exec_source, projected_tools};
use super::*;
use zeta_protocol::{ToolDefinition, ToolName};

#[test]
fn control_catalog_exposes_exec_and_wait_with_stable_names() {
    let definitions = control_definitions();
    assert_eq!(
        definitions
            .iter()
            .map(|definition| definition.name.as_str())
            .collect::<Vec<_>>(),
        vec![EXEC_TOOL_NAME, WAIT_TOOL_NAME]
    );
    assert!(definitions.iter().all(|definition| definition.strict));
}

#[test]
fn code_projection_normalizes_tool_names_and_rejects_collisions() {
    let definitions = vec![
        ToolDefinition {
            name: ToolName::new("Weather-API").unwrap(),
            description: "weather".into(),
            parameters: serde_json::json!({"type": "object"}),
            strict: true,
        },
        ToolDefinition {
            name: ToolName::new("weather__api").unwrap(),
            description: "duplicate projection".into(),
            parameters: serde_json::json!({"type": "object"}),
            strict: true,
        },
    ];
    let error = projected_tools(&definitions).unwrap_err();
    assert!(error.to_string().contains("collision"));
}

#[test]
fn exec_directive_extracts_limits_without_exposing_the_directive_to_javascript() {
    let parsed = parse_exec_source(
        "// @exec: {\"yieldTimeMs\": 250, \"maxOutputTokens\": 32}\ntext(\"ok\");",
    )
    .unwrap();
    assert_eq!(parsed.source, "text(\"ok\");");
    assert_eq!(parsed.yield_time_ms, Some(250));
    assert_eq!(parsed.max_output_tokens, Some(32));
}

#[test]
fn exec_directive_rejects_unknown_options() {
    let error = parse_exec_source("// @exec: {\"timeoutMs\": 1}\ntext(\"ok\");").unwrap_err();
    assert!(error.to_string().contains("unsupported"));
}
