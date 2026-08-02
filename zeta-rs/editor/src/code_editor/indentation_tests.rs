use super::{CodeEditorDocument, CodeEditorIndentation};
use crate::{CodeEditorCommand, CodeEditorPosition};

#[test]
fn newline_preserves_leading_whitespace_and_indents_after_an_opener() {
    let mut document = CodeEditorDocument::from_text("    if ready {");
    document.apply(CodeEditorCommand::MoveToLineEnd(
        crate::CodeEditorSelectionMode::Move,
    ));
    document.apply(CodeEditorCommand::Newline);

    assert_eq!(document.text(), "    if ready {\n        ");
    assert_eq!(document.cursor(), document.text().len());
}

#[test]
fn newline_between_a_delimiter_pair_places_the_caret_on_an_indented_line() {
    let mut document = CodeEditorDocument::from_text("fn main() {}");
    document.set_selection(
        CodeEditorPosition {
            row_index: 0,
            byte_offset: 11,
        },
        CodeEditorPosition {
            row_index: 0,
            byte_offset: 11,
        },
    );
    document.apply(CodeEditorCommand::Newline);

    assert_eq!(document.text(), "fn main() {\n    \n}");
    assert_eq!(&document.text()[..document.cursor()], "fn main() {\n    ");
}

#[test]
fn selected_lines_indent_and_outdent_as_single_undoable_edits() {
    let mut document = CodeEditorDocument::from_text("one\n  two\nthree");
    document.set_indentation(CodeEditorIndentation::spaces(2));
    document.set_selection(
        CodeEditorPosition {
            row_index: 0,
            byte_offset: 1,
        },
        CodeEditorPosition {
            row_index: 2,
            byte_offset: 0,
        },
    );

    document.apply(CodeEditorCommand::Indent);
    assert_eq!(document.text(), "  one\n    two\nthree");
    document.apply(CodeEditorCommand::Undo);
    assert_eq!(document.text(), "one\n  two\nthree");

    document.apply(CodeEditorCommand::Indent);
    document.apply(CodeEditorCommand::Outdent);
    assert_eq!(document.text(), "one\n  two\nthree");
}

#[test]
fn tab_indents_at_a_collapsed_caret_and_shift_tab_removes_line_indentation() {
    let mut document = CodeEditorDocument::from_text("  value");
    document.set_indentation(CodeEditorIndentation::spaces(2));
    document.set_selection(
        CodeEditorPosition {
            row_index: 0,
            byte_offset: 2,
        },
        CodeEditorPosition {
            row_index: 0,
            byte_offset: 2,
        },
    );

    document.apply(CodeEditorCommand::Indent);
    assert_eq!(document.text(), "    value");
    document.apply(CodeEditorCommand::Outdent);
    assert_eq!(document.text(), "  value");
}

#[test]
fn tab_indentation_policy_is_explicit() {
    let mut document = CodeEditorDocument::from_text("{");
    document.set_indentation(CodeEditorIndentation::tabs());
    document.apply(CodeEditorCommand::MoveToLineEnd(
        crate::CodeEditorSelectionMode::Move,
    ));
    document.apply(CodeEditorCommand::Newline);

    assert_eq!(document.text(), "{\n\t");
}
