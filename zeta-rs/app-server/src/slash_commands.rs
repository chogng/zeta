use std::collections::BTreeSet;
use std::fmt;
use zeta_app_server_protocol::protocol::slash_commands::SlashCommandDefinition;

/// Validated server-owned snapshot of slash commands advertised during initialization.
///
/// Hosts construct a complete catalog at the composition boundary. App Server connections receive
/// a clone of that immutable snapshot so one connection cannot observe popup metadata that changes
/// midway through its lifetime.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct SlashCommandCatalog {
    commands: Vec<SlashCommandDefinition>,
}

impl SlashCommandCatalog {
    pub fn new(
        commands: impl IntoIterator<Item = SlashCommandDefinition>,
    ) -> Result<Self, SlashCommandCatalogError> {
        let commands = commands.into_iter().collect::<Vec<_>>();
        let mut names = BTreeSet::new();
        for command in &commands {
            validate_command(command)?;
            if !names.insert(command.name.as_str()) {
                return Err(SlashCommandCatalogError(format!(
                    "duplicate slash command name '{}'",
                    command.name
                )));
            }
        }
        Ok(Self { commands })
    }

    pub(crate) fn definitions(&self) -> &[SlashCommandDefinition] {
        &self.commands
    }
}

/// Failure to construct a server-advertised slash-command snapshot.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SlashCommandCatalogError(pub String);

impl fmt::Display for SlashCommandCatalogError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for SlashCommandCatalogError {}

fn validate_command(command: &SlashCommandDefinition) -> Result<(), SlashCommandCatalogError> {
    if command.name.is_empty()
        || command.name.starts_with('-')
        || command.name.ends_with('-')
        || !command.name.chars().all(|character| {
            character.is_ascii_lowercase() || character.is_ascii_digit() || character == '-'
        })
    {
        return Err(SlashCommandCatalogError(format!(
            "invalid slash command name '{}': use lowercase ASCII letters, digits, and interior hyphens",
            command.name
        )));
    }
    if command.description.trim().is_empty() {
        return Err(SlashCommandCatalogError(format!(
            "slash command '{}' must have a description",
            command.name
        )));
    }
    Ok(())
}

#[cfg(test)]
#[path = "slash_commands_tests.rs"]
mod tests;
