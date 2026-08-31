//! TUI-local product commands contributed to the shared Slash Commands catalog.

use super::state::ChatInputItem;
use super::state::ChatSubmission;
use strum::IntoEnumIterator;
use strum_macros::{AsRefStr, EnumIter, EnumString, IntoStaticStr};
use zeta_slash_commands::SlashCommandArgumentMode;
use zeta_slash_commands::SlashCommandDefinition;
use zeta_slash_commands::SlashCommandInvocation as ParsedSlashCommand;
use zeta_slash_commands::SlashCommandOrigin;

#[cfg(test)]
use zeta_slash_commands::SlashCommandCatalog;

/// TUI execution binding for a locally contributed Slash Command definition.
#[derive(AsRefStr, Clone, Copy, Debug, EnumIter, EnumString, Eq, IntoStaticStr, PartialEq)]
#[strum(serialize_all = "kebab-case")]
pub(crate) enum TuiSlashCommandAction {
    Status,
    #[strum(serialize = "statusline")]
    StatusLine,
    Skills,
    Mcp,
    Resume,
    Archive,
    Connectors,
    Rewind,
    Config,
    AddDir,
    Fork,
    Help,
    Shortcuts,
    Export,
    Model,
    Theme,
    New,
    Quit,
    Sessions,
    Agents,
    Subagents,
    Queue,
}

impl TuiSlashCommandAction {
    pub(crate) fn command(self) -> &'static str {
        self.into()
    }

    pub(crate) fn description(self) -> &'static str {
        match self {
            Self::Status => "show the active session, thread, and model",
            Self::StatusLine => "choose the items shown in the status line",
            Self::Sessions | Self::Agents => "open the Session Manager",
            Self::Subagents => "focus the current Session Thread list",
            Self::Queue => "manage queued messages for the current Thread",
            Self::Skills => "browse configured skill sources",
            Self::Mcp => "list configured MCP tools",
            Self::Connectors => "show external service connections",
            Self::Resume => "list or resume a saved session",
            Self::Archive => "archive the current session and start a new chat",
            Self::Rewind => "return to an earlier message checkpoint",
            Self::Config => "show the current configuration",
            Self::AddDir => "add or manage a session directory",
            Self::Fork => "fork the current chat",
            Self::Help => "show executable slash commands",
            Self::Shortcuts => "browse and customize terminal shortcuts",
            Self::Export => "export this conversation as Markdown",
            Self::Model => "show or set the preferred provider/model",
            Self::Theme => "show or set the terminal color theme",
            Self::New => "start a new chat",
            Self::Quit => "quit Zeta",
        }
    }

    pub(crate) fn argument_mode(self) -> SlashCommandArgumentMode {
        match self {
            Self::Resume
            | Self::Rewind
            | Self::AddDir
            | Self::Fork
            | Self::Export
            | Self::Model
            | Self::Theme
            | Self::New => SlashCommandArgumentMode::Optional,
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

pub(crate) fn built_in_slash_command_definitions() -> Vec<SlashCommandDefinition> {
    TuiSlashCommandAction::iter()
        .map(TuiSlashCommandAction::definition)
        .collect()
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct SlashCommandInvocation {
    pub(crate) command: SlashCommandDefinition,
    pub(crate) origin: SlashCommandOrigin,
    pub(crate) display_arguments: String,
    pub(crate) arguments: Vec<ChatInputItem>,
}

impl SlashCommandInvocation {
    pub(crate) fn display_text(&self) -> String {
        let command = format!("/{}", self.command.name);
        if self.display_arguments.is_empty() {
            command
        } else {
            format!("{command} {}", self.display_arguments)
        }
    }

    pub(crate) fn into_forwarded_submission(mut self) -> ChatSubmission {
        let display_text = self.display_text();
        let command_text = format!("/{}", self.command.name);

        match self.arguments.first_mut() {
            Some(ChatInputItem::Text(text)) => {
                *text = format!("{command_text} {text}");
            }
            Some(ChatInputItem::Image { .. }) | Some(ChatInputItem::Skill { .. }) | None => {
                self.arguments.insert(0, ChatInputItem::Text(command_text));
            }
        }

        ChatSubmission {
            display_text,
            input: self.arguments,
        }
    }
}

pub(super) fn into_command_invocation(
    mut submission: ChatSubmission,
    parsed: ParsedSlashCommand,
) -> Result<SlashCommandInvocation, ChatSubmission> {
    let command_prefix = format!("/{}", parsed.command.name);
    let Some(ChatInputItem::Text(first_text)) = submission.input.first_mut() else {
        return Err(submission);
    };
    let Some(arguments) = first_text.strip_prefix(&command_prefix) else {
        return Err(submission);
    };
    let arguments = arguments.trim_start().to_owned();
    if arguments.is_empty() {
        submission.input.remove(0);
    } else {
        *first_text = arguments;
    }

    Ok(SlashCommandInvocation {
        command: parsed.command,
        origin: parsed.origin,
        display_arguments: submission.display_text[parsed.arguments_range].to_owned(),
        arguments: submission.input,
    })
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
