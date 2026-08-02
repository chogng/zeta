//! TUI-local product commands contributed to the shared Slash Commands catalog.

use strum::IntoEnumIterator;
use strum_macros::{AsRefStr, EnumIter, EnumString, IntoStaticStr};
use zeta_slash_commands::{SlashCommandArgumentMode, SlashCommandDefinition};

#[cfg(test)]
use zeta_slash_commands::SlashCommandCatalog;

/// TUI execution binding for a locally contributed Slash Command definition.
#[derive(AsRefStr, Clone, Copy, Debug, EnumIter, EnumString, Eq, IntoStaticStr, PartialEq)]
#[strum(serialize_all = "kebab-case")]
pub(crate) enum TuiSlashCommandAction {
    Status,
    Skills,
    Mcp,
    Resume,
    Clear,
    Config,
    Fork,
    Help,
    Model,
    Theme,
    New,
    Quit,
    Exit,
}

impl TuiSlashCommandAction {
    pub(crate) fn command(self) -> &'static str {
        self.into()
    }

    pub(crate) fn description(self) -> &'static str {
        match self {
            Self::Status => "show the active session, thread, and model",
            Self::Skills => "browse configured skill sources",
            Self::Mcp => "list configured MCP tools",
            Self::Resume => "list or resume a saved session",
            Self::Clear => "clear the terminal and start a new chat",
            Self::Config => "show the current configuration",
            Self::Fork => "fork the current chat",
            Self::Help => "show executable slash commands",
            Self::Model => "show or set the preferred provider/model",
            Self::Theme => "show or set the terminal color theme",
            Self::New => "start a new chat",
            Self::Quit | Self::Exit => "exit Zeta",
        }
    }

    pub(crate) fn argument_mode(self) -> SlashCommandArgumentMode {
        match self {
            Self::Resume | Self::Clear | Self::Fork | Self::Model | Self::Theme | Self::New => {
                SlashCommandArgumentMode::Optional
            }
            _ => SlashCommandArgumentMode::None,
        }
    }

    pub(crate) fn definition(self) -> SlashCommandDefinition {
        SlashCommandDefinition {
            name: self.command().into(),
            description: self.description().into(),
            argument_mode: self.argument_mode(),
        }
    }
}

pub(crate) fn built_in_slash_commands() -> Vec<(&'static str, TuiSlashCommandAction)> {
    TuiSlashCommandAction::iter()
        .map(|command| (command.command(), command))
        .collect()
}

pub(crate) fn built_in_slash_command_definitions() -> Vec<SlashCommandDefinition> {
    TuiSlashCommandAction::iter()
        .map(TuiSlashCommandAction::definition)
        .collect()
}

#[cfg(test)]
pub(crate) fn default_slash_command_catalog() -> SlashCommandCatalog {
    SlashCommandCatalog::with_local_and_server(
        built_in_slash_command_definitions(),
        std::iter::empty(),
    )
    .expect("the TUI built-in Slash Commands catalog is valid")
}

#[cfg(test)]
pub(crate) fn built_in_catalog_command(command: TuiSlashCommandAction) -> SlashCommandDefinition {
    default_slash_command_catalog()
        .command_named(command.command())
        .expect("the requested TUI built-in command is registered")
        .clone()
}

#[cfg(test)]
#[path = "slash_commands_tests.rs"]
mod tests;
