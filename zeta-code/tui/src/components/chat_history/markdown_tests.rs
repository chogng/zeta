use super::export_markdown;
use super::latest_agent_response;
use crate::components::transcript::Message;
use crate::components::transcript::MessageRole;

#[test]
fn latest_agent_response_ignores_tools_and_notices() {
    let messages = vec![
        Message::plain(MessageRole::Agent, "first".into()),
        Message::plain(MessageRole::Tool, "tool".into()),
        Message::plain(MessageRole::Agent, "second".into()),
        Message::plain(MessageRole::Notice, "done".into()),
    ];

    assert_eq!(latest_agent_response(&messages), Some("second"));
}

#[test]
fn markdown_export_preserves_roles_and_details() {
    let messages = vec![
        Message::plain(MessageRole::User, "hello".into()),
        Message::plain(MessageRole::Agent, "world".into()).with_detail("detail"),
    ];

    assert_eq!(
        export_markdown(&messages),
        "## User\n\nhello\n\n## Zeta\n\nworld\n\n```text\ndetail\n```\n\n"
    );
}
