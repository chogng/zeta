use crate::components::chat_input::ChatSubmission;
use crate::components::chat_input::SlashCommandInvocation;
use crate::components::steer::SteerId;
use crate::features::config::AdditionalDirectoryPermissionEdit;
use crate::features::config::ConfigEdit;
use crate::features::config::ProviderApiKeyEdit;
use crate::features::interactions::InteractionResponse;
use crate::features::keymap::KeymapEdit;
use crate::features::status_line::StatusLineEdit;
use std::path::PathBuf;
use zeta_app_server_protocol::protocol::config::McpServerEnablementDto;
use zeta_app_server_protocol::protocol::skills::SkillEnablementDto;
use zeta_protocol::SkillId;
use zeta_protocol::TurnId;

/// A typed side-effect intent emitted by the single-writer application state.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum AppCommand {
    ExecuteProductCommand(SlashCommandInvocation),
    Quit,
    Interrupt,
    Suspend,
    CopyLastResponse,
    OpenConfigPane,
    OpenKeymapPane,
    OpenStatusLinePane,
    EditKeymap(KeymapEdit),
    EditConfig(ConfigEdit),
    EditAdditionalDirectoryPermissions(AdditionalDirectoryPermissionEdit),
    SetProviderApiKey(ProviderApiKeyEdit),
    EditStatusLine(StatusLineEdit),
    ConnectConnectorDeviceOAuth {
        connector_id: String,
        connection_generation: u64,
    },
    DisconnectConnector {
        connector_id: String,
    },
    ExportTranscript {
        requested_path: Option<PathBuf>,
    },
    LoadOlderHistory,
    ReadClipboardImage,
    OpenCustomThemePane,
    OpenRewindPane,
    RemoveAdditionalDirectory {
        root: PathBuf,
    },
    RewindToCheckpoint {
        before_turn_id: TurnId,
        checkpoint_label: String,
    },
    ResumeSession {
        session_id: String,
    },
    ResolveInteraction(InteractionResponse),
    SetMcpEnablement {
        server_id: String,
        enablement: McpServerEnablementDto,
    },
    SetPreferredModel {
        preference: String,
    },
    SetCustomTheme {
        preference: String,
    },
    SetTheme {
        preference: String,
    },
    SetSkillEnablement {
        skill_id: SkillId,
        enablement: SkillEnablementDto,
    },
    CycleNextApprovalMode,
    SubmitTurn {
        submission: ChatSubmission,
    },
    SteerTurn {
        steer_id: SteerId,
        submission: ChatSubmission,
    },
}
