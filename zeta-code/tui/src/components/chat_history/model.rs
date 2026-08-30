#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum MessageRole {
    User,
    Agent,
    Reasoning,
    Plan,
    Command,
    Notice,
    Error,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum CommandStatus {
    Running,
    Succeeded,
    Failed,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Message {
    pub(crate) cell_id: Option<String>,
    pub(crate) render_revision: u64,
    pub(crate) role: MessageRole,
    pub(crate) text: String,
    pub(crate) detail: Option<String>,
    pub(crate) command_status: Option<CommandStatus>,
    pub(crate) can_expand: bool,
    pub(crate) expanded: bool,
    pub(crate) has_details: bool,
    pub(crate) selected: bool,
}

impl Message {
    pub(crate) fn plain(role: MessageRole, text: String) -> Self {
        Self {
            cell_id: None,
            render_revision: 0,
            role,
            text,
            detail: None,
            command_status: None,
            can_expand: false,
            expanded: false,
            has_details: false,
            selected: false,
        }
    }

    pub(crate) fn command(command: String, status: CommandStatus, detail: Option<String>) -> Self {
        Self {
            cell_id: None,
            render_revision: 0,
            role: MessageRole::Command,
            text: command,
            detail,
            command_status: Some(status),
            can_expand: false,
            expanded: false,
            has_details: false,
            selected: false,
        }
    }

    pub(crate) fn with_cell_id(mut self, cell_id: impl Into<String>) -> Self {
        self.cell_id = Some(cell_id.into());
        self
    }

    pub(crate) fn with_render_revision(mut self, revision: u64) -> Self {
        self.render_revision = revision;
        self
    }

    pub(crate) fn with_detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = Some(detail.into());
        self
    }

    pub(crate) fn with_cell_actions(
        mut self,
        can_expand: bool,
        expanded: bool,
        has_details: bool,
        selected: bool,
    ) -> Self {
        self.can_expand = can_expand;
        self.expanded = expanded;
        self.has_details = has_details;
        self.selected = selected;
        self
    }
}
