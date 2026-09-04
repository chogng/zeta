//! Stable plain-Markdown export for the terminal transcript.

use super::Message;
use super::MessageRole;

pub(crate) fn export_markdown(messages: &[Message]) -> String {
    let mut output = String::new();
    for message in messages {
        output.push_str("## ");
        output.push_str(role_label(message.role));
        output.push_str("\n\n");
        output.push_str(&message.text);
        output.push('\n');
        if let Some(detail) = &message.detail {
            output.push_str("\n```text\n");
            output.push_str(detail);
            if !detail.ends_with('\n') {
                output.push('\n');
            }
            output.push_str("```\n");
        }
        output.push('\n');
    }
    output
}

fn role_label(role: MessageRole) -> &'static str {
    match role {
        MessageRole::User => "User",
        MessageRole::Agent => "Zeta",
        MessageRole::Reasoning => "Reasoning",
        MessageRole::Plan => "Plan",
        MessageRole::Command => "Command",
        MessageRole::Notice => "Notice",
        MessageRole::Error => "Error",
    }
}

#[cfg(test)]
#[path = "markdown_tests.rs"]
mod tests;
