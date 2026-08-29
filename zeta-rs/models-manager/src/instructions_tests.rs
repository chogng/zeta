use super::*;

#[test]
fn base_instructions_have_stable_metadata() {
    assert_eq!(BASE_INSTRUCTIONS.owner(), "models-manager");
    assert_eq!(BASE_INSTRUCTIONS.id(), "model/base-instructions");
    assert_eq!(BASE_INSTRUCTIONS.revision(), "model-base-v1");
    assert!(!BASE_INSTRUCTIONS.body().trim().is_empty());
    assert!(BASE_INSTRUCTIONS.body().ends_with('\n'));
}

#[test]
fn base_instructions_defer_tool_schemas_to_the_host() {
    assert!(
        BASE_INSTRUCTIONS
            .body()
            .contains("Follow the host-provided tool definitions and schemas exactly")
    );
    assert!(!BASE_INSTRUCTIONS.body().contains("read_file"));
    assert!(!BASE_INSTRUCTIONS.body().contains("apply_patch"));
}
