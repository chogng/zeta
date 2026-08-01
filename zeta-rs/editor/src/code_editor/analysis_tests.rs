use super::*;

#[test]
fn shell_analysis_tracks_unicode_edits_incrementally() {
    let mut analysis = CodeEditorAnalysis::default();
    analysis.set_language(CodeEditorLanguage::Shell);
    let initial = "echo \"你好\"";
    let initial_line = 0..initial.len();
    assert!(!analysis.synchronize(initial, std::slice::from_ref(&initial_line))[0].is_empty());

    let changed = "echo \"你好世界\"";
    let changed_line = 0..changed.len();
    let tokens = analysis.synchronize(changed, std::slice::from_ref(&changed_line));
    assert!(tokens[0].iter().any(|token| token.range == (5..19)));
}

#[test]
fn plain_text_clears_parser_state_and_tokens() {
    let mut analysis = CodeEditorAnalysis::default();
    analysis.set_language(CodeEditorLanguage::Rust);
    let text = "pub fn main() {}";
    let line = 0..text.len();
    assert!(!analysis.synchronize(text, std::slice::from_ref(&line))[0].is_empty());
    analysis.set_language(CodeEditorLanguage::PlainText);
    assert!(analysis.synchronize(text, std::slice::from_ref(&line))[0].is_empty());
}
