use crate::components::composer::DynamicSlashCommand;
use crate::components::composer::SlashCommandArgumentMode;
use crate::components::composer::SlashCommandRegistry;
use zeta_app_server_client::ClientError;
use zeta_app_server_protocol::protocol::slash_commands::{
    SlashCommandArgumentModeDto, SlashCommandDefinition,
};

pub(crate) fn slash_command_registry(
    definitions: &[SlashCommandDefinition],
) -> Result<SlashCommandRegistry, ClientError> {
    let commands = definitions.iter().map(|definition| DynamicSlashCommand {
        name: definition.name.clone(),
        description: definition.description.clone(),
        argument_mode: match definition.argument_mode {
            SlashCommandArgumentModeDto::None => SlashCommandArgumentMode::None,
            SlashCommandArgumentModeDto::Optional => SlashCommandArgumentMode::Optional,
        },
    });
    SlashCommandRegistry::with_dynamic_commands(commands).map_err(|error| {
        ClientError::Protocol(format!(
            "App Server advertised an invalid slash command snapshot: {error}"
        ))
    })
}
