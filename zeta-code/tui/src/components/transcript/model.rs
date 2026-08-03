#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum MessageRole {
    User,
    Agent,
    Command,
    Notice,
    Error,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum CommandStatus {
    Running,
    Succeeded,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Message {
    pub(crate) role: MessageRole,
    pub(crate) text: String,
    pub(crate) detail: Option<String>,
    pub(crate) command_status: Option<CommandStatus>,
}

impl Message {
    pub(crate) fn plain(role: MessageRole, text: String) -> Self {
        Self {
            role,
            text,
            detail: None,
            command_status: None,
        }
    }

    pub(crate) fn command(command: String, status: CommandStatus, detail: Option<String>) -> Self {
        Self {
            role: MessageRole::Command,
            text: command,
            detail,
            command_status: Some(status),
        }
    }
}
