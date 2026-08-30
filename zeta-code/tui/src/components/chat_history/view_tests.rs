use super::ChatHistoryPointerTarget;
use super::ChatHistoryView;
use super::message_lines;
use super::pointer_target_at;
use crate::components::chat_history::ChatHistoryScroll;
use crate::components::chat_history::CommandStatus;
use crate::components::chat_history::Message;
use crate::components::chat_history::MessageRole;
use crate::components::welcome::WelcomeModel;
use crate::render::Renderable;
use crate::render::test_context;
use ratatui::layout::Rect;
use ratatui::style::Color;
use std::path::Path;

#[test]
fn tool_output_renders_ansi_as_styled_spans() {
    let messages = vec![
        Message::command("shell · stdout".into(), CommandStatus::Running, None)
            .with_detail("plain \x1b[31mred\x1b[0m"),
    ];

    let lines = message_lines(&messages, test_context());
    let output = &lines[1];
    let visible = output
        .spans
        .iter()
        .map(|span| span.content.as_ref() as &str)
        .collect::<String>();

    assert_eq!(visible, "└─ plain red");
    assert!(
        output
            .spans
            .iter()
            .any(|span| span.content == "red" && span.style.fg == Some(Color::Red))
    );
    assert!(!visible.contains('\x1b'));
}

#[test]
fn renderable_measurement_uses_the_same_wrapped_message_rows_as_drawing() {
    let messages = vec![Message::plain(
        MessageRole::Agent,
        "a response that wraps at narrow widths".into(),
    )];
    let scroll = ChatHistoryScroll::default();
    let welcome = WelcomeModel::for_workspace(Path::new("."));
    let view = ChatHistoryView {
        messages: &messages,
        scroll: &scroll,
        welcome: &welcome,
        presentation_highlight: test_context().highlight(),
    };

    assert!(view.desired_height(12, test_context()) > view.desired_height(80, test_context()));
}

#[test]
fn multiline_content_uses_the_same_continuation_prefix_for_measurement_and_drawing() {
    let messages = vec![Message::plain(
        MessageRole::Agent,
        "first line\nsecond line".into(),
    )];

    let lines = message_lines(&messages, test_context());

    assert_eq!(lines[0].to_string(), "◆  first line");
    assert_eq!(lines[1].to_string(), "   second line");
}

#[test]
fn multiline_command_output_keeps_detail_prefix_alignment() {
    let messages = vec![
        Message::command("printf hi".into(), CommandStatus::Succeeded, None)
            .with_detail("one\ntwo"),
    ];

    let lines = message_lines(&messages, test_context());

    assert_eq!(lines[1].to_string(), "└─ one");
    assert_eq!(lines[2].to_string(), "   two");
}

#[test]
fn pointer_rows_follow_the_same_multiline_height_as_rendering() {
    let messages = vec![
        Message::plain(MessageRole::Agent, "first\nsecond".into()),
        Message::plain(MessageRole::Reasoning, "Thought".into())
            .with_cell_id("reasoning")
            .with_cell_actions(true, false, false, false),
    ];

    assert_eq!(
        pointer_target_at(
            Rect::new(0, 0, 30, 10),
            &messages,
            &ChatHistoryScroll::default(),
            test_context(),
            2,
            3,
        ),
        Some(ChatHistoryPointerTarget::Toggle("reasoning".into()))
    );
}
