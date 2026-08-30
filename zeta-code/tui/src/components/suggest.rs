mod mention;
mod skill;
mod view;

use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
pub(crate) use mention::MentionMatchKind;
pub(crate) use mention::MentionPluginItem;
pub(crate) use mention::MentionPopupView;
use mention::Mentions;
use skill::SkillSelector;
pub(crate) use skill::SkillSelectorItem;
pub(crate) use skill::SkillSelectorView;
use std::ops::Range;
pub(crate) use view::draw;
pub(crate) use view::index_at;
use zeta_file_search::PathSearchSnapshot;
use zeta_protocol::SkillRef;
use zeta_slash_commands::SlashCommandCatalog;
use zeta_slash_commands::SlashCommandDefinition;
use zeta_slash_commands::SlashCommandInvocation;
use zeta_slash_commands::SlashCommandsState;
use zeta_slash_commands::SlashCommandsView;

#[derive(Debug)]
pub(crate) struct Suggest {
    slash_commands: SlashCommandsState,
    mentions: Mentions,
    skills: SkillSelector,
}

#[derive(Debug)]
pub(in crate::components) enum SuggestInputOutcome {
    Completed(SuggestEdit),
    Consumed,
    Submit(SuggestEdit),
    Unhandled,
}

#[derive(Debug)]
pub(in crate::components) enum SuggestEdit {
    Text {
        range: Range<usize>,
        replacement: String,
    },
    Element {
        range: Range<usize>,
        value: String,
        skill: Option<SkillRef>,
    },
}

pub(crate) enum SuggestView<'a> {
    Slash(SlashCommandsView<'a>),
    Mention(MentionPopupView<'a>),
    Skill(SkillSelectorView<'a>),
}

impl Suggest {
    pub(in crate::components) fn new(slash_commands: SlashCommandCatalog) -> Self {
        Self {
            slash_commands: SlashCommandsState::new(slash_commands),
            mentions: Mentions::default(),
            skills: SkillSelector::default(),
        }
    }

    pub(in crate::components) fn handle_key(&mut self, key: KeyEvent) -> SuggestInputOutcome {
        if self.skills.view().is_some() {
            return match key.code {
                KeyCode::Esc => {
                    self.skills.dismiss();
                    SuggestInputOutcome::Consumed
                }
                KeyCode::Up => {
                    self.skills.select_previous();
                    SuggestInputOutcome::Consumed
                }
                KeyCode::Down => {
                    self.skills.select_next();
                    SuggestInputOutcome::Consumed
                }
                KeyCode::Tab => self
                    .skills
                    .complete_selected()
                    .map(skill_edit)
                    .map(SuggestInputOutcome::Completed)
                    .unwrap_or(SuggestInputOutcome::Consumed),
                KeyCode::Enter if key.modifiers.is_empty() => self
                    .skills
                    .complete_selected()
                    .map(skill_edit)
                    .map(SuggestInputOutcome::Completed)
                    .unwrap_or(SuggestInputOutcome::Unhandled),
                _ => SuggestInputOutcome::Unhandled,
            };
        }

        if self.mentions.view().is_some() {
            return match key.code {
                KeyCode::Esc => {
                    self.mentions.dismiss();
                    SuggestInputOutcome::Consumed
                }
                KeyCode::Up => {
                    self.mentions.select_previous();
                    SuggestInputOutcome::Consumed
                }
                KeyCode::Down => {
                    self.mentions.select_next();
                    SuggestInputOutcome::Consumed
                }
                KeyCode::Tab => self
                    .mentions
                    .complete_selected()
                    .map(mention_edit)
                    .map(SuggestInputOutcome::Completed)
                    .unwrap_or(SuggestInputOutcome::Consumed),
                KeyCode::Enter if key.modifiers.is_empty() => self
                    .mentions
                    .complete_selected()
                    .map(mention_edit)
                    .map(SuggestInputOutcome::Completed)
                    .unwrap_or(SuggestInputOutcome::Unhandled),
                _ => SuggestInputOutcome::Unhandled,
            };
        }

        if self.slash_commands.view().is_some() {
            return match key.code {
                KeyCode::Esc => {
                    self.slash_commands.dismiss();
                    SuggestInputOutcome::Consumed
                }
                KeyCode::Up => {
                    self.slash_commands.select_previous();
                    SuggestInputOutcome::Consumed
                }
                KeyCode::Down => {
                    self.slash_commands.select_next();
                    SuggestInputOutcome::Consumed
                }
                KeyCode::Tab => self
                    .slash_commands
                    .selected_command()
                    .and_then(|command| self.slash_edit(command))
                    .map(SuggestInputOutcome::Completed)
                    .unwrap_or(SuggestInputOutcome::Consumed),
                KeyCode::Enter if key.modifiers.is_empty() => self
                    .slash_commands
                    .selected_command()
                    .and_then(|command| self.slash_edit(command))
                    .map(SuggestInputOutcome::Submit)
                    .unwrap_or(SuggestInputOutcome::Unhandled),
                _ => SuggestInputOutcome::Unhandled,
            };
        }

        SuggestInputOutcome::Unhandled
    }

    pub(in crate::components) fn sync_textarea(
        &mut self,
        text: &str,
        cursor: usize,
        slash_command_element_active: bool,
    ) {
        self.mentions.sync(text, cursor);
        self.skills.sync(text, cursor);
        if slash_command_element_active {
            self.slash_commands.clear();
        } else {
            self.slash_commands.sync_input(text, cursor);
        }
    }

    pub(in crate::components) fn view(&self) -> Option<SuggestView<'_>> {
        self.slash_commands
            .view()
            .map(SuggestView::Slash)
            .or_else(|| self.mentions.view().map(SuggestView::Mention))
            .or_else(|| self.skills.view().map(SuggestView::Skill))
    }

    pub(in crate::components) fn mention_query(&self) -> Option<&str> {
        self.mentions.query()
    }

    pub(in crate::components) fn apply_file_search_snapshot(
        &mut self,
        snapshot: PathSearchSnapshot,
    ) {
        self.mentions.apply_search_snapshot(snapshot);
    }

    pub(in crate::components) fn replace_catalog(
        &mut self,
        slash_commands: SlashCommandCatalog,
        skills: Vec<SkillSelectorItem>,
        plugins: Vec<MentionPluginItem>,
    ) {
        self.slash_commands.set_catalog(slash_commands);
        self.skills.replace_catalog(skills);
        self.mentions.replace_plugin_catalog(plugins);
    }

    pub(in crate::components) fn activate(&mut self, index: usize) -> Option<SuggestInputOutcome> {
        match self.view()? {
            SuggestView::Slash(_) => {
                let command = self.slash_commands.command_at(index)?.clone();
                self.slash_edit(&command).map(SuggestInputOutcome::Submit)
            }
            SuggestView::Mention(_) => self
                .mentions
                .complete_at(index)
                .map(mention_edit)
                .map(SuggestInputOutcome::Completed),
            SuggestView::Skill(_) => self
                .skills
                .complete_at(index)
                .map(skill_edit)
                .map(SuggestInputOutcome::Completed),
        }
    }

    pub(in crate::components) fn select(&mut self, index: usize) -> bool {
        match self.view() {
            Some(SuggestView::Slash(_)) => self.slash_commands.select(index),
            Some(SuggestView::Mention(_)) => self.mentions.select(index),
            Some(SuggestView::Skill(_)) => self.skills.select(index),
            None => false,
        }
    }

    pub(in crate::components) fn invocation(&self, text: &str) -> Option<SlashCommandInvocation> {
        self.slash_commands.invocation(text)
    }

    pub(in crate::components) fn command_element_range(
        &self,
        text: &str,
        cursor: usize,
    ) -> Option<Range<usize>> {
        self.slash_commands.command_element_range(text, cursor)
    }

    pub(in crate::components) fn clear(&mut self) {
        self.slash_commands.clear();
        self.mentions.clear();
        self.skills.clear();
    }

    fn slash_edit(&self, command: &SlashCommandDefinition) -> Option<SuggestEdit> {
        let completion = self.slash_commands.completion(command)?;
        Some(SuggestEdit::Text {
            range: completion.range,
            replacement: completion.replacement,
        })
    }
}

fn mention_edit(completion: mention::MentionCompletion) -> SuggestEdit {
    SuggestEdit::Element {
        range: completion.range,
        value: completion.value,
        skill: None,
    }
}

fn skill_edit(completion: skill::SkillCompletion) -> SuggestEdit {
    SuggestEdit::Element {
        range: completion.range,
        value: completion.value,
        skill: Some(completion.skill),
    }
}
