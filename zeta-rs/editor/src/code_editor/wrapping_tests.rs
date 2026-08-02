use super::*;
use crate::{CodeEditorDocument, CodeEditorRowSource};

#[test]
fn wrapping_preserves_graphemes_tabs_and_empty_rows() {
    let document = CodeEditorDocument::from_text("ab界cd\na\tb\n");
    let projection = CodeEditorVisualProjection::new(&document, CodeEditorLineWrapping::Soft, 4);

    assert_eq!(projection.len(), 5);
    assert_eq!(projection.line(0).unwrap().start_byte, 0);
    assert_eq!(projection.line(0).unwrap().end_byte, "ab界".len());
    assert_eq!(projection.line(1).unwrap().start_byte, "ab界".len());
    assert_eq!(projection.line(2).unwrap().end_column, 4);
    assert_eq!(projection.line(3).unwrap().start_column, 4);
    assert_eq!(projection.line(4).unwrap().start_byte, 0);
    assert_eq!(document.row_count(), 3);
}
