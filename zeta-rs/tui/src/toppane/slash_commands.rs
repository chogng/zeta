use strum::IntoEnumIterator;
use strum_macros::AsRefStr;
use strum_macros::EnumIter;
use strum_macros::EnumString;
use strum_macros::IntoStaticStr;

/// Whether a slash command accepts structured inline arguments after its name.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SlashCommandArgumentMode {
    None,
    Optional,
}

/// Local commands recognized by the chat composer.
///
/// The composer identifies commands, while application coordination remains responsible for
/// executing them. Product operations that require App Server requests should be added only after
/// their typed protocol flow exists.
#[derive(AsRefStr, Clone, Copy, Debug, EnumIter, EnumString, Eq, IntoStaticStr, PartialEq)]
#[strum(serialize_all = "kebab-case")]
pub(crate) enum SlashCommand {
    // Enum order is popup presentation order.
    Status,
    Skills,
    Mcp,
    Resume,
    Clear,
    Config,
    Fork,
    Help,
    Model,
    New,
    Quit,
    Exit,
}

impl SlashCommand {
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
            Self::New => "start a new chat",
            Self::Quit | Self::Exit => "exit Zeta",
        }
    }

    pub(crate) fn argument_mode(self) -> SlashCommandArgumentMode {
        match self {
            Self::Resume | Self::Clear | Self::Fork | Self::Model | Self::New => {
                SlashCommandArgumentMode::Optional
            }
            _ => SlashCommandArgumentMode::None,
        }
    }
}

/// Runtime-provided command metadata accepted by [`SlashCommandRegistry`].
///
/// Dynamic commands share popup and parsing semantics with built-ins. The application coordinator
/// remains responsible for assigning them an execution path.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct DynamicSlashCommand {
    pub(crate) name: String,
    pub(crate) description: String,
    pub(crate) argument_mode: SlashCommandArgumentMode,
}

/// One command visible to slash discovery and submission parsing.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum SlashCommandItem {
    Builtin(SlashCommand),
    Dynamic(DynamicSlashCommand),
}

impl SlashCommandItem {
    pub(crate) fn command(&self) -> &str {
        match self {
            Self::Builtin(command) => command.command(),
            Self::Dynamic(command) => &command.name,
        }
    }

    pub(crate) fn description(&self) -> &str {
        match self {
            Self::Builtin(command) => command.description(),
            Self::Dynamic(command) => &command.description,
        }
    }

    pub(crate) fn argument_mode(&self) -> SlashCommandArgumentMode {
        match self {
            Self::Builtin(command) => command.argument_mode(),
            Self::Dynamic(command) => command.argument_mode,
        }
    }
}

/// Canonical command view shared by popup discovery, completion, and submission parsing.
///
/// Runtime command sources must replace this registry as one validated snapshot so the popup
/// cannot display a command that submission parsing would reject.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct SlashCommandRegistry {
    commands: Vec<SlashCommandItem>,
}

impl SlashCommandRegistry {
    pub(crate) fn with_dynamic_commands(
        dynamic_commands: impl IntoIterator<Item = DynamicSlashCommand>,
    ) -> Result<Self, String> {
        let mut commands = built_in_slash_commands()
            .into_iter()
            .map(|(_, command)| SlashCommandItem::Builtin(command))
            .collect::<Vec<_>>();

        for command in dynamic_commands {
            validate_dynamic_command(&command)?;
            if commands
                .iter()
                .any(|registered| registered.command() == command.name)
            {
                return Err(format!("duplicate slash command name '{}'", command.name));
            }
            commands.push(SlashCommandItem::Dynamic(command));
        }

        Ok(Self { commands })
    }

    pub(super) fn matching(&self, prefix: &str) -> Vec<SlashCommandItem> {
        self.commands
            .iter()
            .filter(|command| command.command().starts_with(prefix))
            .cloned()
            .collect()
    }

    pub(crate) fn command_named(&self, name: &str) -> Option<SlashCommandItem> {
        self.commands
            .iter()
            .find(|command| command.command() == name)
            .cloned()
    }
}

impl Default for SlashCommandRegistry {
    fn default() -> Self {
        Self::with_dynamic_commands(std::iter::empty())
            .expect("the built-in slash command registry is valid")
    }
}

/// Returns built-in commands in popup presentation order.
pub(crate) fn built_in_slash_commands() -> Vec<(&'static str, SlashCommand)> {
    SlashCommand::iter()
        .map(|command| (command.command(), command))
        .collect()
}

fn validate_dynamic_command(command: &DynamicSlashCommand) -> Result<(), String> {
    if command.name.is_empty()
        || command.name.starts_with('-')
        || command.name.ends_with('-')
        || !command.name.chars().all(|character| {
            character.is_ascii_lowercase() || character.is_ascii_digit() || character == '-'
        })
    {
        return Err(format!(
            "invalid slash command name '{}': use lowercase ASCII letters, digits, and interior hyphens",
            command.name
        ));
    }
    if command.description.trim().is_empty() {
        return Err(format!(
            "slash command '{}' must have a description",
            command.name
        ));
    }
    Ok(())
}

#[cfg(test)]
#[path = "slash_commands_tests.rs"]
mod tests;
