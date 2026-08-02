use crate::components::composer::ComposerSubmission;
use crate::components::composer::SlashCommandInvocation;
use zeta_app_server_protocol::protocol::skills::SkillEnablementDto;
use zeta_protocol::SkillId;

/// A typed side-effect intent emitted by the single-writer application state.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum AppCommand {
    ExecuteProductCommand(SlashCommandInvocation),
    Quit,
    Interrupt,
    ReadClipboardImage,
    OpenCustomThemePane,
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
