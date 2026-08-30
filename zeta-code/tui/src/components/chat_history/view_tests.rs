use super::message_lines;
use crate::components::chat_history::CommandStatus;
use crate::components::chat_history::Message;
use ratatui::style::Color;

#[test]
fn tool_output_renders_ansi_as_styled_spans() {
    let messages = vec![
        Message::command("shell · stdout".into(), CommandStatus::Running, None)
            .with_detail("plain \x1b[31mred\x1b[0m"),
    ];

    let lines = message_lines(&messages);
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
