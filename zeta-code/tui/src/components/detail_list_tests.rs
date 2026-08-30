use super::DetailList;
use super::DetailListRow;

#[test]
fn detail_list_exposes_read_only_rows_without_selection_state() {
    let detail = DetailList::new("Status", vec![DetailListRow::new("Model", "openai/gpt")]);

    assert_eq!(detail.title(), "Status");
    assert_eq!(detail.rows()[0].label(), "Model");
    assert_eq!(detail.rows()[0].value(), "openai/gpt");
}
