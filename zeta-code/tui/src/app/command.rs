use crate::components::composer::ComposerSubmission;
use crate::components::composer::SlashCommandInvocation;
use crate::features::interactions::InteractionResponse;
use std::path::PathBuf;
use zeta_app_server_protocol::protocol::config::McpServerEnablementDto;
use zeta_app_server_protocol::protocol::skills::SkillEnablementDto;
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
    SubmitTurn(ComposerSubmission),
}
