#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum MessageRole {
    User,
    Agent,
    Reasoning,
    Plan,
    Tool,
    ToolError,
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
    pub(crate) source_id: Option<String>,
    pub(crate) role: MessageRole,
    pub(crate) text: String,
    pub(crate) detail: Option<String>,
    pub(crate) command_status: Option<CommandStatus>,
}

impl Message {
    pub(crate) fn plain(role: MessageRole, text: String) -> Self {
        Self {
            source_id: None,
            role,
            text,
            detail: None,
            command_status: None,
        }
    }

    pub(crate) fn command(command: String, status: CommandStatus, detail: Option<String>) -> Self {
        Self {
            source_id: None,
            role: MessageRole::Command,
            text: command,
            detail,
            command_status: Some(status),
        }
    }

    pub(crate) fn with_source_id(mut self, source_id: impl Into<String>) -> Self {
        self.source_id = Some(source_id.into());
        self
    }

    pub(crate) fn with_detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = Some(detail.into());
        self
    }
}
