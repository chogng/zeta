use crate::components::composer::SlashCommandCatalog;
use crate::components::composer::built_in_slash_command_definitions;
use zeta_app_server_client::ClientError;
use zeta_app_server_protocol::protocol::slash_commands::SlashCommandDefinition;

pub(crate) fn slash_command_registry(
    definitions: &[SlashCommandDefinition],
) -> Result<SlashCommandCatalog, ClientError> {
    SlashCommandCatalog::with_local_and_server(
        built_in_slash_command_definitions(),
        definitions.iter().cloned(),
    )
    .map_err(|error| {
        ClientError::Protocol(format!(
            "App Server advertised an invalid slash command snapshot: {error}"
        ))
    })
}
