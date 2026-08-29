mod mention;
mod skill;
mod view;

pub(crate) use mention::MentionMatchKind;
pub(crate) use mention::MentionPluginItem;
pub(crate) use mention::MentionPopupView;
pub(super) use mention::Mentions;
pub(super) use skill::SkillSelector;
pub(crate) use skill::SkillSelectorItem;
pub(crate) use skill::SkillSelectorView;
pub(crate) use view::draw;
pub(crate) use view::index_at;
use zeta_slash_commands::SlashCommandsView;

pub(crate) enum SuggestView<'a> {
    Slash(SlashCommandsView<'a>),
    Mention(MentionPopupView<'a>),
    Skill(SkillSelectorView<'a>),
}
