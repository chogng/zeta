use super::ChatHistoryView;
use super::message_lines;
use crate::components::chat_history::ChatHistoryScroll;
use crate::components::chat_history::CommandStatus;
use crate::components::chat_history::Message;
use crate::components::chat_history::MessageRole;
use crate::components::welcome::WelcomeModel;
use crate::render::Renderable;
use crate::render::test_context;
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

    assert!(view.desired_height(12) > view.desired_height(80));
}
