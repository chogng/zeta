use crate::components::chat_input::ChatSubmission;
use crate::components::chat_input::SlashCommandInvocation;
use crate::components::steer::SteerId;
use crate::features::config::ConfigEdit;
use crate::features::config::PermissionEdit;
use crate::features::config::ProviderApiKeyEdit;
use crate::features::keymap::KeymapEdit;
use crate::features::queue::QueueId;
use crate::features::status_line::StatusLineEdit;
use crate::features::thread::ThreadRequestResponse;
use std::path::PathBuf;
use zeta_app_server_protocol::protocol::config::McpServerEnablementDto;
use zeta_app_server_protocol::protocol::skills::SkillEnablementDto;
use zeta_protocol::SessionId;
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
    OpenConfigEditor,
    OpenKeymapEditor,
    OpenStatusLineRegion,
    OpenThemePicker,
    EditKeymap(KeymapEdit),
    EditConfig(ConfigEdit),
    EditPermissions(PermissionEdit),
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
    OpenCustomThemePicker,
    OpenRewindRegion,
    RemoveDir {
        path: PathBuf,
    },
    RewindToCheckpoint {
        before_turn_id: TurnId,
        checkpoint_label: String,
    },
    ResumeSession {
        session_id: String,
        preferred_thread_id: Option<ThreadId>,
    },
    ArchiveSessions {
        session_ids: Vec<SessionId>,
    },
    CreateSessionAndEnter {
        submission: ChatSubmission,
    },
    SwitchThread {
        thread_id: ThreadId,
    },
    ResolveThreadRequest(ThreadRequestResponse),
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
    SubmitQueuedTurn {
        queue_id: QueueId,
        submission: ChatSubmission,
    },
    SteerTurn {
        steer_id: SteerId,
        submission: ChatSubmission,
    },
}
