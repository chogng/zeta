use super::ComposerShellSyntax;
use zeta_editor::{CodeEditorSyntaxHighlighter, CodeEditorTokenRole};

#[test]
fn shell_snapshot_projects_just_as_a_code_editor_token() {
    let mut syntax = ComposerShellSyntax::new();
    let projection = syntax
        .synchronize("just native-dev")
        .expect("Shell analysis should succeed");

    let tokens = projection.highlight_line(1, "just native-dev");

    assert!(
        tokens
            .iter()
            .any(|token| token.range == (0..4) && token.role == CodeEditorTokenRole::Function)
    );
}

#[test]
fn shell_document_applies_unicode_edits_incrementally() {
    let mut syntax = ComposerShellSyntax::new();
    syntax
        .synchronize("echo \"你好\"")
        .expect("initial Shell analysis should succeed");

    let projection = syntax
        .synchronize("echo \"你好世界\"")
        .expect("incremental Shell analysis should succeed");

    assert!(
        projection
            .highlight_line(1, "echo \"你好世界\"")
            .iter()
            .any(|token| token.range == (5..19))
    );
}
