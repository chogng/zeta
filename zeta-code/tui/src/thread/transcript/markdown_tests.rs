use super::export_markdown;
use crate::thread::transcript::Message;
use crate::thread::transcript::MessageRole;

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
