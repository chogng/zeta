use super::attachments::Attachments;
use super::attachments::ImagePasteOutcome;
use super::editor::TextArea;
use super::editor::TextAreaOutcome;
use super::editor::TextElementId;
use super::mentions::MentionPopupView;
use super::mentions::Mentions;
use super::pending_pastes::PendingPastes;
use super::skills::SkillSelector;
use super::skills::SkillSelectorItem;
use super::skills::SkillSelectorView;
use super::wrap::wrap_input;
use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use crossterm::event::KeyModifiers;
use zeta_file_search::PathSearchSnapshot;
use zeta_protocol::SkillRef;
use zeta_slash_commands::{
    SlashCommandCatalog, SlashCommandDefinition, SlashCommandInvocation as ParsedSlashCommand,
    SlashCommandOrigin, SlashCommandsState, SlashCommandsView,
};

const MAX_COMPOSER_HISTORY: usize = 100;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum ComposerOutcome {
    Command(SlashCommandInvocation),
    Consumed,
    Submit(ComposerSubmission),
    Unhandled,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum ComposerInput {
    Text(String),
    Image { url: String },
    Skill { skill: SkillRef },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ComposerSubmission {
    pub(crate) display_text: String,
    pub(crate) input: Vec<ComposerInput>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct SlashCommandInvocation {
    pub(crate) command: SlashCommandDefinition,
    pub(crate) origin: SlashCommandOrigin,
    pub(crate) display_arguments: String,
    pub(crate) arguments: Vec<ComposerInput>,
}

impl SlashCommandInvocation {
    pub(crate) fn into_forwarded_submission(mut self) -> ComposerSubmission {
        let command_text = format!("/{}", self.command.name);
        let display_text = if self.display_arguments.is_empty() {
            command_text.clone()
        } else {
            format!("{command_text} {}", self.display_arguments)
        };

        match self.arguments.first_mut() {
            Some(ComposerInput::Text(text)) => {
                *text = format!("{command_text} {text}");
            }
            Some(ComposerInput::Image { .. }) | Some(ComposerInput::Skill { .. }) | None => {
                self.arguments.insert(0, ComposerInput::Text(command_text));
            }
        }

        ComposerSubmission {
            display_text,
            input: self.arguments,
        }
    }
}

/// Owns chat-input semantics around an editing-oriented [`TextArea`].
///
/// Slash parsing belongs to `zeta-slash-commands`; this component applies its completion plans, owns popup
/// key routing, and turns parsed submissions into command invocations. The text area remains
/// responsible only for editing state and keymap behavior.
#[derive(Debug)]
pub(crate) struct ChatComposer {
    textarea: TextArea,
    slash_commands: SlashCommandsState,
    slash_command_element: Option<TextElementId>,
    mentions: Mentions,
    skills: SkillSelector,
    pending_pastes: PendingPastes,
    attachments: Attachments,
    history: Vec<String>,
    history_index: Option<usize>,
    history_draft: String,
}

impl ChatComposer {
    #[cfg(test)]
    pub(crate) fn new() -> Self {
        Self::with_slash_commands(super::default_slash_command_catalog())
    }

    pub(crate) fn with_slash_commands(slash_commands: SlashCommandCatalog) -> Self {
        Self {
            textarea: TextArea::new(),
            slash_commands: SlashCommandsState::new(slash_commands),
            slash_command_element: None,
            mentions: Mentions::default(),
            skills: SkillSelector::default(),
            pending_pastes: PendingPastes::default(),
            attachments: Attachments::default(),
            history: Vec::new(),
            history_index: None,
            history_draft: String::new(),
        }
    }

    pub(crate) fn handle_key(&mut self, key: KeyEvent) -> ComposerOutcome {
        if self.skills.view().is_some() {
            match key.code {
                KeyCode::Esc => {
                    self.skills.dismiss();
                    return ComposerOutcome::Consumed;
                }
                KeyCode::Up => {
                    self.skills.select_previous();
                    return ComposerOutcome::Consumed;
                }
                KeyCode::Down => {
                    self.skills.select_next();
                    return ComposerOutcome::Consumed;
                }
                KeyCode::Tab => {
                    self.skills.complete_selected(&mut self.textarea);
                    self.sync_after_text_change();
                    return ComposerOutcome::Consumed;
                }
                KeyCode::Enter
                    if key.modifiers.is_empty()
                        && self.skills.complete_selected(&mut self.textarea) =>
                {
                    self.sync_after_text_change();
                    return ComposerOutcome::Consumed;
                }
                _ => {}
            }
        }

        if self.mentions.view().is_some() {
            match key.code {
                KeyCode::Esc => {
                    self.mentions.dismiss();
                    return ComposerOutcome::Consumed;
                }
                KeyCode::Up => {
                    self.mentions.select_previous();
                    return ComposerOutcome::Consumed;
                }
                KeyCode::Down => {
                    self.mentions.select_next();
                    return ComposerOutcome::Consumed;
                }
                KeyCode::Tab => {
                    self.mentions.complete_selected(&mut self.textarea);
                    self.sync_after_text_change();
                    return ComposerOutcome::Consumed;
                }
                KeyCode::Enter
                    if key.modifiers.is_empty()
                        && self.mentions.complete_selected(&mut self.textarea) =>
                {
                    self.sync_after_text_change();
                    return ComposerOutcome::Consumed;
                }
                _ => {}
            }
        }

        if self.slash_commands.view().is_some() {
            match key.code {
                KeyCode::Esc => {
                    self.slash_commands.dismiss();
                    return ComposerOutcome::Consumed;
                }
                KeyCode::Up => {
                    self.slash_commands.select_previous();
                    return ComposerOutcome::Consumed;
                }
                KeyCode::Down => {
                    self.slash_commands.select_next();
                    return ComposerOutcome::Consumed;
                }
                KeyCode::Tab => {
                    if let Some(command) = self.slash_commands.selected_command().cloned() {
                        self.complete_slash_command(&command);
                    }
                    return ComposerOutcome::Consumed;
                }
                KeyCode::Enter if key.modifiers.is_empty() => {
                    if let Some(command) = self.slash_commands.selected_command().cloned() {
                        self.complete_slash_command(&command);
                        return self.submit();
                    }
                }
                _ => {}
            }
        }

        if is_newline_key(key) {
            self.reset_history_navigation();
            self.textarea.insert_newline();
            self.sync_after_text_change();
            return ComposerOutcome::Consumed;
        }
        if key.code == KeyCode::Enter && key.modifiers.is_empty() {
            return self.submit();
        }
        match key.code {
            KeyCode::Up if !self.textarea.can_move_up() => return self.previous_history(),
            KeyCode::Down if !self.textarea.can_move_down() => return self.next_history(),
            _ => {}
        }

        match self.textarea.handle_key(key) {
            TextAreaOutcome::Consumed => {
                self.reset_history_navigation();
                self.sync_after_text_change();
                ComposerOutcome::Consumed
            }
            TextAreaOutcome::Unhandled => ComposerOutcome::Unhandled,
        }
    }

    #[cfg(test)]
    pub(crate) fn insert_text(&mut self, text: &str) {
        self.reset_history_navigation();
        self.textarea.insert_text(text);
        self.sync_after_text_change();
    }

    pub(crate) fn handle_paste(&mut self, pasted: String) -> Result<(), String> {
        self.reset_history_navigation();
        match self
            .attachments
            .try_attach_pasted_path(&mut self.textarea, &pasted)
        {
            ImagePasteOutcome::Attached => {}
            ImagePasteOutcome::NotImage => {
                self.pending_pastes.insert_text(&mut self.textarea, pasted)
            }
            ImagePasteOutcome::Rejected(error) => return Err(error),
        }
        self.sync_after_text_change();
        Ok(())
    }

    pub(crate) fn attach_image_bytes(&mut self, bytes: Vec<u8>) -> Result<(), String> {
        self.reset_history_navigation();
        self.attachments
            .attach_image_bytes(&mut self.textarea, bytes)?;
        self.sync_after_text_change();
        Ok(())
    }

    pub(crate) fn text(&self) -> &str {
        self.textarea.text()
    }

    pub(crate) fn cursor_display_width(&self) -> usize {
        self.textarea.cursor_display_width()
    }

    pub(crate) fn cursor_line(&self) -> usize {
        self.textarea.cursor_line()
    }

    pub(crate) fn desired_height(&self, available_width: u16) -> u16 {
        const MAX_VISIBLE_LINES: usize = 6;
        let rows = wrap_input(
            self.textarea.text(),
            self.textarea.cursor_line(),
            self.textarea.cursor_display_width(),
            available_width,
        )
        .lines
        .len()
        .min(MAX_VISIBLE_LINES);
        u16::try_from(rows.saturating_add(2)).unwrap_or(u16::MAX)
    }

    pub(crate) fn slash_popup(&self) -> Option<SlashCommandsView<'_>> {
        self.slash_commands.view()
    }

    pub(crate) fn mention_popup(&self) -> Option<MentionPopupView<'_>> {
        self.mentions.view()
    }

    pub(crate) fn skill_popup(&self) -> Option<SkillSelectorView<'_>> {
        self.skills.view()
    }

    pub(crate) fn mention_query(&self) -> Option<&str> {
        self.mentions.query()
    }

    pub(crate) fn apply_file_search_snapshot(&mut self, snapshot: PathSearchSnapshot) {
        self.mentions.apply_search_snapshot(snapshot);
    }

    pub(crate) fn activate_slash_command(&mut self, index: usize) -> Option<ComposerOutcome> {
        let command = self.slash_commands.command_at(index)?.clone();
        self.complete_slash_command(&command);
        Some(self.submit())
    }

    pub(crate) fn select_slash_command(&mut self, index: usize) -> bool {
        self.slash_commands.select(index)
    }

    pub(crate) fn activate_mention(&mut self, index: usize) -> bool {
        let completed = self.mentions.complete_at(&mut self.textarea, index);
        if completed {
            self.sync_after_text_change();
        }
        completed
    }

    pub(crate) fn select_mention(&mut self, index: usize) -> bool {
        self.mentions.select(index)
    }

    pub(crate) fn activate_skill(&mut self, index: usize) -> bool {
        let completed = self.skills.complete_at(&mut self.textarea, index);
        if completed {
            self.sync_after_text_change();
        }
        completed
    }

    pub(crate) fn select_skill(&mut self, index: usize) -> bool {
        self.skills.select(index)
    }

    pub(crate) fn replace_composer_catalog(
        &mut self,
        slash_commands: SlashCommandCatalog,
        skills: Vec<SkillSelectorItem>,
    ) {
        self.slash_commands.set_catalog(slash_commands);
        self.skills.replace_catalog(skills);
        self.sync_after_text_change();
    }

    fn submit(&mut self) -> ComposerOutcome {
        let Some(submission) = self.prepare_submission() else {
            return ComposerOutcome::Consumed;
        };
        if submission
            .input
            .iter()
            .all(|input| matches!(input, ComposerInput::Text(_)))
        {
            self.record_history(submission.display_text.clone());
        }
        let command = self.slash_commands.invocation(&submission.display_text);
        self.clear();
        match command {
            Some(command) => match into_command_invocation(submission, command) {
                Ok(invocation) => ComposerOutcome::Command(invocation),
                Err(submission) => ComposerOutcome::Submit(submission),
            },
            None => ComposerOutcome::Submit(submission),
        }
    }

    fn prepare_submission(&self) -> Option<ComposerSubmission> {
        let raw_text = self.textarea.text();
        let display_text = self.pending_pastes.expand(&self.textarea);
        let display_text = display_text.trim().to_owned();
        let mut input = Vec::new();
        let mut selected_skills = Vec::new();
        let mut text = String::new();
        let mut cursor = 0;

        for (element_id, range) in self.textarea.elements() {
            text.push_str(&raw_text[cursor..range.start]);
            if let Some(replacement) = self.pending_pastes.replacement(element_id) {
                text.push_str(replacement);
            } else if let Some(url) = self.attachments.image_url(element_id) {
                push_text_input(&mut input, &mut text);
                input.push(ComposerInput::Image {
                    url: url.to_owned(),
                });
            } else {
                text.push_str(&raw_text[range.clone()]);
                if let Some(skill) = self.skills.skill_for(element_id)
                    && !selected_skills.contains(skill)
                {
                    selected_skills.push(skill.clone());
                }
            }
            cursor = range.end;
        }
        text.push_str(&raw_text[cursor..]);
        push_text_input(&mut input, &mut text);
        input.splice(
            0..0,
            selected_skills
                .into_iter()
                .map(|skill| ComposerInput::Skill { skill }),
        );

        (!input.is_empty()).then_some(ComposerSubmission {
            display_text,
            input,
        })
    }

    fn clear(&mut self) {
        self.textarea.clear();
        self.slash_commands.clear();
        self.slash_command_element = None;
        self.mentions.clear();
        self.skills.clear();
        self.pending_pastes.clear();
        self.attachments.clear();
        self.reset_history_navigation();
    }

    fn complete_slash_command(&mut self, command: &SlashCommandDefinition) -> bool {
        let Some(completion) = self.slash_commands.completion(command) else {
            return false;
        };
        self.textarea
            .replace_range(completion.range, &completion.replacement);
        self.sync_after_text_change();
        true
    }

    fn sync_after_text_change(&mut self) {
        self.pending_pastes.retain_present_in(&self.textarea);
        self.attachments.reconcile(&mut self.textarea);
        self.sync_slash_command_element();
        self.mentions.sync_textarea(&self.textarea);
        self.skills.sync_textarea(&self.textarea);
        if self.slash_command_element.is_some() {
            self.slash_commands.clear();
        } else {
            self.slash_commands
                .sync_input(self.textarea.text(), self.textarea.cursor());
        }
    }

    fn sync_slash_command_element(&mut self) {
        let desired = self
            .slash_commands
            .command_element_range(self.textarea.text(), self.textarea.cursor());

        if let Some(element_id) = self.slash_command_element {
            let current = self.textarea.element_range(element_id);
            if current.is_none() || current != desired {
                self.textarea.unmark_element(element_id);
                self.slash_command_element = None;
            }
        }
        if self.slash_command_element.is_none()
            && let Some(range) = desired
        {
            self.slash_command_element = Some(self.textarea.mark_element(range));
        }
    }

    fn previous_history(&mut self) -> ComposerOutcome {
        if self.history.is_empty() {
            return ComposerOutcome::Consumed;
        }
        let index = match self.history_index {
            Some(index) => index.saturating_sub(1),
            None => {
                self.history_draft = self.textarea.text().to_owned();
                self.history.len() - 1
            }
        };
        self.history_index = Some(index);
        self.replace_with_history_entry(index);
        ComposerOutcome::Consumed
    }

    fn next_history(&mut self) -> ComposerOutcome {
        let Some(index) = self.history_index else {
            return ComposerOutcome::Consumed;
        };
        if index + 1 < self.history.len() {
            self.history_index = Some(index + 1);
            self.replace_with_history_entry(index + 1);
        } else {
            self.history_index = None;
            self.textarea.replace_text(&self.history_draft);
            self.history_draft.clear();
            self.sync_after_text_change();
        }
        ComposerOutcome::Consumed
    }

    fn replace_with_history_entry(&mut self, index: usize) {
        self.textarea.replace_text(&self.history[index]);
        self.pending_pastes.clear();
        self.attachments.clear();
        self.slash_command_element = None;
        self.mentions.clear();
        self.skills.clear();
        self.slash_commands.clear();
        self.sync_after_text_change();
    }

    fn record_history(&mut self, entry: String) {
        if self.history.last() != Some(&entry) {
            self.history.push(entry);
            if self.history.len() > MAX_COMPOSER_HISTORY {
                self.history.remove(0);
            }
        }
    }

    fn reset_history_navigation(&mut self) {
        self.history_index = None;
        self.history_draft.clear();
    }
}

fn into_command_invocation(
    mut submission: ComposerSubmission,
    parsed: ParsedSlashCommand,
) -> Result<SlashCommandInvocation, ComposerSubmission> {
    let command_prefix = format!("/{}", parsed.command.name);
    let Some(ComposerInput::Text(first_text)) = submission.input.first_mut() else {
        return Err(submission);
    };
    let Some(arguments) = first_text.strip_prefix(&command_prefix) else {
        return Err(submission);
    };
    let arguments = arguments.trim_start().to_owned();
    if arguments.is_empty() {
        submission.input.remove(0);
    } else {
        *first_text = arguments;
    }

    Ok(SlashCommandInvocation {
        command: parsed.command,
        origin: parsed.origin,
        display_arguments: submission.display_text[parsed.arguments_range].to_owned(),
        arguments: submission.input,
    })
}

fn push_text_input(input: &mut Vec<ComposerInput>, text: &mut String) {
    let text = std::mem::take(text);
    let text = text.trim();
    if !text.is_empty() {
        input.push(ComposerInput::Text(text.to_owned()));
    }
}

fn is_newline_key(key: KeyEvent) -> bool {
    (matches!(key.code, KeyCode::Enter)
        && key
            .modifiers
            .intersects(KeyModifiers::SHIFT | KeyModifiers::ALT))
        || matches!(key.code, KeyCode::Char('j')) && key.modifiers == KeyModifiers::CONTROL
}

#[cfg(test)]
#[path = "state_tests.rs"]
mod tests;
