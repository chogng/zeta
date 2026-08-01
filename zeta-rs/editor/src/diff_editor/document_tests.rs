use zeta_diff::DiffDocument;

use super::*;

#[test]
fn document_owns_language_and_both_syntax_snapshots() {
    let diff = DiffDocument::from_text(
        "fn before() { println!(\"old\"); }\n",
        "fn after() { println!(\"new\"); }\n",
    )
    .unwrap();
    let mut document = DiffEditorDocument::new(diff, CodeEditorLanguage::Rust);

    assert_eq!(document.language(), CodeEditorLanguage::Rust);
    assert!(
        !document
            .syntax_tokens(DiffEditorSide::Original, 1)
            .is_empty()
    );
    assert!(
        !document
            .syntax_tokens(DiffEditorSide::Modified, 1)
            .is_empty()
    );

    document.set_language(CodeEditorLanguage::PlainText);
    assert!(
        document
            .syntax_tokens(DiffEditorSide::Original, 1)
            .is_empty()
    );
    assert!(
        document
            .syntax_tokens(DiffEditorSide::Modified, 1)
            .is_empty()
    );
}
