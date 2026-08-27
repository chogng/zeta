//! Requests sent to the product command service.

use crate::AppCommandId;

/// A request to execute one stable app command.
///
/// Callers should pass this value across module boundaries instead of calling
/// another product module directly. Arguments can be added to this request in
/// a typed form when a real command needs them; the command identity remains
/// stable and separate from the product state that executes it.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct CommandRequest {
    command_id: AppCommandId,
}

impl CommandRequest {
    /// Creates a request for `command_id`.
    pub const fn new(command_id: AppCommandId) -> Self {
        Self { command_id }
    }

    /// Returns the stable command identity carried by this request.
    pub const fn command_id(self) -> AppCommandId {
        self.command_id
    }
}

impl From<AppCommandId> for CommandRequest {
    fn from(command_id: AppCommandId) -> Self {
        Self::new(command_id)
    }
}
