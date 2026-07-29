#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum MessageRole {
    User,
    Agent,
    Notice,
    Error,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Message {
    pub(crate) role: MessageRole,
    pub(crate) text: String,
}
