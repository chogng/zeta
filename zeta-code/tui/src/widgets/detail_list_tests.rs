use super::DetailList;
use super::DetailListRow;
use super::draw_scrolled;
use crate::render::test_context;
use ratatui::Terminal;
use ratatui::backend::TestBackend;

#[test]
fn detail_list_exposes_read_only_rows_without_selection_state() {
    let detail = DetailList::new("Status", vec![DetailListRow::new("Model", "openai/gpt")]);

    assert_eq!(detail.title(), "Status");
    assert_eq!(detail.rows()[0].label(), "Model");
    assert_eq!(detail.rows()[0].value(), "openai/gpt");
}

#[test]
fn detail_list_renders_each_value_line_below_its_label() {
    let detail = DetailList::new(
        "Process",
        vec![DetailListRow::new("Content", "first line\nsecond line")],
    );
    let mut terminal = Terminal::new(TestBackend::new(30, 6)).unwrap();

    terminal
        .draw(|frame| {
            draw_scrolled(frame, frame.area(), &detail, 0, test_context());
        })
        .unwrap();

    let rendered = terminal.backend().to_string();
    assert!(rendered.contains("Content: first line"));
    assert!(rendered.contains("         second line"));
}

#[test]
fn detail_list_wraps_long_values_instead_of_truncating_them() {
    let detail = DetailList::new(
        "Process",
        vec![DetailListRow::new(
            "Output",
            "a process result that is wider than the detail surface",
        )],
    );
    let mut terminal = Terminal::new(TestBackend::new(24, 7)).unwrap();

    assert!(detail.desired_height_for_width(20) > detail.desired_height_for_width(80));

    terminal
        .draw(|frame| {
            draw_scrolled(frame, frame.area(), &detail, 0, test_context());
        })
        .unwrap();

    let rendered = terminal.backend().to_string();
    assert!(rendered.contains("Output: a process"));
    assert!(rendered.contains("surface"));
}
