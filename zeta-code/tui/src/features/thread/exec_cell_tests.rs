use super::ExecCell;
use super::ExecGroup;
use crate::components::chat_history::CommandStatus;
use crate::components::chat_history::ExecutionKind;
use zeta_protocol::ToolCallId;
use zeta_protocol::ToolName;
use zeta_protocol::ToolOutputStream;

#[test]
fn exploration_calls_group_by_stable_tool_classification() {
    let mut cell = ExecCell::start(
        "read-entry".into(),
        call_id("read"),
        &tool_name("read_file"),
        "{}".into(),
    );
    assert!(cell.can_accept(&tool_name("search")));
    cell.push_call(
        "search-entry".into(),
        call_id("search"),
        &tool_name("search"),
        "{}".into(),
    );

    assert_eq!(cell.group, ExecGroup::ExploreGroup);
    assert_eq!(cell.view(false).text, "Explored 2 operations");
}

#[test]
fn output_and_result_route_to_the_exact_tool_call() {
    let first = call_id("first");
    let second = call_id("second");
    let mut cell = ExecCell::start(
        "first-entry".into(),
        first.clone(),
        &tool_name("read"),
        "{}".into(),
    );
    cell.push_call(
        "second-entry".into(),
        second.clone(),
        &tool_name("search"),
        "{}".into(),
    );
    cell.apply_output(
        "second-output".into(),
        &second,
        ToolOutputStream::Stdout,
        "second only".into(),
    );
    cell.complete("first-result".into(), &first, "first done".into(), false);
    cell.complete("second-result".into(), &second, "second done".into(), true);

    assert_eq!(cell.view(false).command_status, Some(CommandStatus::Failed));
    let detail = cell.full_details();
    assert!(detail.contains("second only"));
    assert!(detail.contains("first done"));
    assert!(detail.contains("second done"));
}

#[test]
fn live_output_is_bounded_with_an_omission_marker() {
    let call = call_id("bounded");
    let mut cell = ExecCell::start(
        "entry".into(),
        call.clone(),
        &tool_name("exec"),
        "{}".into(),
    );
    let output = (0..500)
        .map(|index| format!("line {index} {}", "x".repeat(500)))
        .collect::<Vec<_>>()
        .join("\n");
    cell.apply_output("output".into(), &call, ToolOutputStream::Stdout, output);

    let detail = cell.full_details();
    assert!(detail.len() < 70 * 1024);
    assert!(detail.contains("omitted"));
}

#[test]
fn execution_kind_distinguishes_commands_mutations_and_neutral_tools() {
    let command = ExecCell::start(
        "command-entry".into(),
        call_id("command"),
        &tool_name("shell-command"),
        "{}".into(),
    );
    let mutation = ExecCell::start(
        "mutation-entry".into(),
        call_id("mutation"),
        &tool_name("write_file"),
        "{}".into(),
    );
    let read = ExecCell::start(
        "read-entry".into(),
        call_id("read-kind"),
        &tool_name("read_file"),
        "{}".into(),
    );

    assert_eq!(command.view(false).execution_kind, ExecutionKind::Command);
    assert_eq!(mutation.view(false).execution_kind, ExecutionKind::Mutation);
    assert_eq!(read.view(false).execution_kind, ExecutionKind::Neutral);
}

fn call_id(value: &str) -> ToolCallId {
    ToolCallId::new(value).expect("the test ToolCall ID is valid")
}

fn tool_name(value: &str) -> ToolName {
    ToolName::new(value).expect("the test Tool name is valid")
}
