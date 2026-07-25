use super::estimated_wrapped_rows;

#[test]
fn wrapped_row_estimate_accounts_for_the_role_label() {
    assert_eq!(estimated_wrapped_rows(5, "hello", 10), 1);
    assert_eq!(estimated_wrapped_rows(5, "hello!", 10), 2);
}

#[test]
fn wrapped_row_estimate_uses_terminal_width_for_wide_characters() {
    assert_eq!(estimated_wrapped_rows(0, "你好", 4), 1);
    assert_eq!(estimated_wrapped_rows(0, "你好呀", 4), 2);
}

#[test]
fn wrapped_row_estimate_handles_an_unrenderable_width() {
    assert_eq!(estimated_wrapped_rows(5, "hello", 0), 0);
}
