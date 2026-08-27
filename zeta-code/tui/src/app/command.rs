use crate::components::composer::ComposerSubmission;
use crate::components::composer::SlashCommandInvocation;
use crate::features::interactions::InteractionResponse;
use crate::features::shortcuts::ShortcutEdit;
use crate::features::status_line::StatusLineEdit;
use std::path::PathBuf;
use zeta_app_server_protocol::protocol::config::McpServerEnablementDto;
use zeta_app_server_protocol::protocol::skills::SkillEnablementDto;
use zeta_protocol::ApprovalMode;
use zeta_protocol::SkillId;
use zeta_protocol::ThreadId;
use zeta_protocol::TurnId;

/// A typed side-effect intent emitted by the single-writer application state.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum AppCommand {
    ExecuteProductCommand(SlashCommandInvocation),
    Quit,
    Interrupt,
    Suspend,
    CopyLastResponse,
    OpenShortcutsPane,
    OpenStatusLinePane,
    EditShortcut(ShortcutEdit),
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
    OpenWorkspaceDirectory {
        path: PathBuf,
    },
    PreviewWorkspaceFile {
        path: PathBuf,
    },
    RewindToCheckpoint {
        before_turn_id: TurnId,
        checkpoint_label: String,
    },
    ResumeSession {
        session_id: String,
    },
    SwitchThread {
        thread_id: ThreadId,
    },
    ArchiveThread {
        thread_id: ThreadId,
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
    SubmitTurn {
        submission: ComposerSubmission,
        approval_mode: ApprovalMode,
    },
}
