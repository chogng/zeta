use super::new_command_id;

#[test]
fn new_logical_commands_receive_distinct_prefixed_ids() {
    let first = new_command_id("turn");
    let second = new_command_id("turn");

    assert!(first.as_str().starts_with("turn-"));
    assert!(second.as_str().starts_with("turn-"));
    assert_ne!(first, second);
}
