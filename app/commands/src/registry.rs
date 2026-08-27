//! Runtime-free command handler registration.

use std::collections::HashMap;

use crate::CommandRequest;
use crate::AppCommandId;

/// A product-owned handler function registered for a stable command.
///
/// The context type is supplied by the host. This keeps the registry
/// independent of `NativeApp` while allowing each host to keep its product
/// state and domain services as the handler context.
pub type CommandHandler<Context> = fn(&mut Context, &CommandRequest);

/// Errors returned when a command registry cannot complete an operation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CommandRegistryError {
    /// The command already has a handler in this registry.
    AlreadyRegistered(AppCommandId),
    /// The request refers to a command without a registered handler.
    NotRegistered(AppCommandId),
}

/// Maps stable command identities to host-owned handlers.
///
/// The registry owns only registration and lookup. It does not own product
/// state, UI objects, or handler lifetimes beyond the function pointers that
/// the host registers during initialization.
#[derive(Debug)]
pub struct CommandRegistry<Context> {
    handlers: HashMap<AppCommandId, CommandHandler<Context>>,
}

impl<Context> Default for CommandRegistry<Context> {
    fn default() -> Self {
        Self::new()
    }
}

impl<Context> CommandRegistry<Context> {
    /// Creates an empty command registry.
    pub fn new() -> Self {
        Self {
            handlers: HashMap::new(),
        }
    }

    /// Registers one handler and rejects duplicate command ownership.
    pub fn register(
        &mut self,
        command_id: AppCommandId,
        handler: CommandHandler<Context>,
    ) -> Result<(), CommandRegistryError> {
        if self.handlers.contains_key(&command_id) {
            return Err(CommandRegistryError::AlreadyRegistered(command_id));
        }
        self.handlers.insert(command_id, handler);
        Ok(())
    }

    /// Returns the registered handler without executing it.
    pub fn handler(&self, command_id: AppCommandId) -> Option<CommandHandler<Context>> {
        self.handlers.get(&command_id).copied()
    }

    /// Executes a request against the supplied host context.
    pub fn execute(
        &self,
        context: &mut Context,
        request: &CommandRequest,
    ) -> Result<(), CommandRegistryError> {
        let handler = self
            .handler(request.command_id())
            .ok_or(CommandRegistryError::NotRegistered(request.command_id()))?;
        handler(context, request);
        Ok(())
    }

    /// Returns whether the registry owns a handler for `command_id`.
    pub fn contains(&self, command_id: AppCommandId) -> bool {
        self.handlers.contains_key(&command_id)
    }

    /// Returns the number of registered command handlers.
    pub fn len(&self) -> usize {
        self.handlers.len()
    }

    /// Returns whether no command handlers have been registered.
    pub fn is_empty(&self) -> bool {
        self.handlers.is_empty()
    }
}

#[cfg(test)]
#[path = "command_registry_tests.rs"]
mod tests;
