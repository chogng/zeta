use super::attachments::Attachments;
use super::attachments::ImagePasteOutcome;
use super::editor::TextArea;
use super::editor::TextAreaOutcome;
use super::editor::TextElementId;
use super::pending_pastes::PendingPastes;
use super::vim::ChatInputMode;
use super::vim::VimOutcome;
use super::vim::VimState;
use super::wrap::wrap_input;
use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use crossterm::event::KeyModifiers;
use std::ops::Range;
use zeta_protocol::SkillRef;
use zeta_slash_commands::{
    SlashCommandDefinition, SlashCommandInvocation as ParsedSlashCommand, SlashCommandOrigin,
};

const MAX_COMPOSER_HISTORY: usize = 100;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum ChatInputOutcome {
    Command(SlashCommandInvocation),
    Consumed,
    Submit(ChatSubmission),
    Unhandled,
}

#[derive(Debug, Eq, PartialEq)]
pub(crate) enum ChatInputQueueOutcome {
    Command(SlashCommandInvocation),
    Consumed,
    Queued(QueuedChatInput),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum ChatInputItem {
    Text(String),
    Image { url: String },
    Skill { skill: SkillRef },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ChatSubmission {
    pub(crate) display_text: String,
    pub(crate) input: Vec<ChatInputItem>,
}

#[derive(Debug, Eq, PartialEq)]
pub(crate) struct QueuedChatInput {
    submission: ChatSubmission,
    draft: ChatInputDraft,
}

impl QueuedChatInput {
    pub(crate) fn display_text(&self) -> &str {
        &self.submission.display_text
    }

    pub(crate) fn submission(&self) -> &ChatSubmission {
        &self.submission
    }
}

#[derive(Debug, Eq, PartialEq)]
struct ChatInputDraft {
    textarea: TextArea,
    vim: VimState,
    slash_command_element: Option<TextElementId>,
    skill_bindings: Vec<(TextElementId, SkillRef)>,
    pending_pastes: PendingPastes,
    attachments: Attachments,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct SlashCommandInvocation {
    pub(crate) command: SlashCommandDefinition,
    pub(crate) origin: SlashCommandOrigin,
    pub(crate) display_arguments: String,
    pub(crate) arguments: Vec<ChatInputItem>,
}

impl SlashCommandInvocation {
    pub(crate) fn into_forwarded_submission(mut self) -> ChatSubmission {
        let command_text = format!("/{}", self.command.name);
        let display_text = if self.display_arguments.is_empty() {
            command_text.clone()
        } else {
            format!("{command_text} {}", self.display_arguments)
        };

        match self.arguments.first_mut() {
            Some(ChatInputItem::Text(text)) => {
                *text = format!("{command_text} {text}");
            }
            Some(ChatInputItem::Image { .. }) | Some(ChatInputItem::Skill { .. }) | None => {
                self.arguments.insert(0, ChatInputItem::Text(command_text));
            }
        }

        ChatSubmission {
            display_text,
            input: self.arguments,
        }
    }
}

/// Owns chat-input semantics around an editing-oriented [`TextArea`].
///
/// Slash parsing and candidate interaction belong to [`Suggest`]; this component owns the editable
/// draft and turns submissions into typed input.
#[derive(Debug)]
pub(crate) struct ChatInput {
    textarea: TextArea,
    input_mode: ChatInputMode,
    vim: VimState,
    slash_command_element: Option<TextElementId>,
    skill_bindings: Vec<(TextElementId, SkillRef)>,
    pending_pastes: PendingPastes,
    attachments: Attachments,
    history: Vec<String>,
    history_index: Option<usize>,
    history_draft: String,
}

impl ChatInput {
    pub(crate) fn new() -> Self {
        Self {
            textarea: TextArea::new(),
            input_mode: ChatInputMode::Standard,
            vim: VimState::default(),
            slash_command_element: None,
            skill_bindings: Vec::new(),
            pending_pastes: PendingPastes::default(),
            attachments: Attachments::default(),
            history: Vec::new(),
            history_index: None,
            history_draft: String::new(),
        }
    }

    pub(in crate::components) fn handle_key(&mut self, key: KeyEvent) -> ChatInputOutcome {
        if self.input_mode == ChatInputMode::Vim
            && self.vim.handle_key(&mut self.textarea, key) == VimOutcome::Consumed
        {
            self.reset_history_navigation();
            return ChatInputOutcome::Consumed;
        }
        if is_newline_key(key) && self.accepts_submission_key() {
            self.reset_history_navigation();
            self.textarea.insert_newline();
            return ChatInputOutcome::Consumed;
        }
        match key.code {
            KeyCode::Up if !self.textarea.can_move_up() => return self.previous_history(),
            KeyCode::Down if !self.textarea.can_move_down() => return self.next_history(),
            _ => {}
        }

        match self.textarea.handle_key(key) {
            TextAreaOutcome::Consumed => {
                self.reset_history_navigation();
                ChatInputOutcome::Consumed
            }
            TextAreaOutcome::Unhandled => ChatInputOutcome::Unhandled,
        }
    }

    #[cfg(test)]
    pub(crate) fn insert_text(&mut self, text: &str) {
        self.reset_history_navigation();
        self.textarea.insert_text(text);
    }

    pub(in crate::components) fn handle_paste(&mut self, pasted: String) -> Result<(), String> {
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
        Ok(())
    }

    pub(in crate::components) fn attach_image_bytes(
        &mut self,
        bytes: Vec<u8>,
    ) -> Result<(), String> {
        self.reset_history_navigation();
        self.attachments
            .attach_image_bytes(&mut self.textarea, bytes)?;
        Ok(())
    }

    pub(crate) fn text(&self) -> &str {
        self.textarea.text()
    }

    pub(crate) fn set_input_mode(&mut self, input_mode: ChatInputMode) {
        if self.input_mode == input_mode {
            return;
        }
        self.input_mode = input_mode;
        self.vim.reset_draft();
    }

    pub(in crate::components) fn accepts_submission_key(&self) -> bool {
        self.input_mode == ChatInputMode::Standard || self.vim.accepts_submission_key()
    }

    pub(crate) fn prompt(&self) -> &'static str {
        match self.input_mode {
            ChatInputMode::Standard => "❯ ",
            ChatInputMode::Vim => self.vim.prompt(),
        }
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

    pub(in crate::components) fn submit_current(
        &mut self,
        command: Option<ParsedSlashCommand>,
    ) -> ChatInputOutcome {
        let Some(submission) = self.prepare_submission() else {
            return ChatInputOutcome::Consumed;
        };
        self.record_submission_history(&submission);
        self.clear();
        match command {
            Some(command) => match into_command_invocation(submission, command) {
                Ok(invocation) => ChatInputOutcome::Command(invocation),
                Err(submission) => ChatInputOutcome::Submit(submission),
            },
            None => ChatInputOutcome::Submit(submission),
        }
    }

    pub(crate) fn queue_current(
        &mut self,
        command: Option<ParsedSlashCommand>,
    ) -> ChatInputQueueOutcome {
        let Some(submission) = self.prepare_submission() else {
            return ChatInputQueueOutcome::Consumed;
        };
        if let Some(command) = command {
            match into_command_invocation(submission.clone(), command) {
                Ok(invocation) if invocation.origin == SlashCommandOrigin::Local => {
                    self.record_submission_history(&submission);
                    self.clear();
                    return ChatInputQueueOutcome::Command(invocation);
                }
                Ok(invocation) => {
                    self.record_submission_history(&submission);
                    let draft = self.take_draft();
                    return ChatInputQueueOutcome::Queued(QueuedChatInput {
                        submission: invocation.into_forwarded_submission(),
                        draft,
                    });
                }
                Err(_) => {}
            }
        }

        self.record_submission_history(&submission);
        let draft = self.take_draft();
        ChatInputQueueOutcome::Queued(QueuedChatInput { submission, draft })
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.prepare_submission().is_none()
    }

    pub(in crate::components) fn submission_display_text(&self) -> Option<String> {
        self.prepare_submission()
            .map(|submission| submission.display_text)
    }

    pub(crate) fn restore_queued(
        &mut self,
        queued: QueuedChatInput,
    ) -> Result<(), Box<QueuedChatInput>> {
        if !self.is_empty() {
            return Err(Box::new(queued));
        }
        let QueuedChatInput { draft, .. } = queued;
        self.clear();
        self.textarea = draft.textarea;
        self.vim = draft.vim;
        self.slash_command_element = draft.slash_command_element;
        self.skill_bindings = draft.skill_bindings;
        self.pending_pastes = draft.pending_pastes;
        self.attachments = draft.attachments;
        Ok(())
    }

    pub(crate) fn submission_contains_skill(&self) -> bool {
        self.prepare_submission().is_some_and(|submission| {
            submission
                .input
                .iter()
                .any(|item| matches!(item, ChatInputItem::Skill { .. }))
        })
    }

    fn prepare_submission(&self) -> Option<ChatSubmission> {
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
                input.push(ChatInputItem::Image {
                    url: url.to_owned(),
                });
            } else {
                text.push_str(&raw_text[range.clone()]);
                if let Some(skill) = self
                    .skill_bindings
                    .iter()
                    .find(|(candidate, _)| *candidate == element_id)
                    .map(|(_, skill)| skill)
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
                .map(|skill| ChatInputItem::Skill { skill }),
        );

        (!input.is_empty()).then_some(ChatSubmission {
            display_text,
            input,
        })
    }

    fn take_draft(&mut self) -> ChatInputDraft {
        let draft = ChatInputDraft {
            textarea: std::mem::replace(&mut self.textarea, TextArea::new()),
            vim: std::mem::take(&mut self.vim),
            slash_command_element: self.slash_command_element.take(),
            skill_bindings: std::mem::take(&mut self.skill_bindings),
            pending_pastes: std::mem::take(&mut self.pending_pastes),
            attachments: std::mem::take(&mut self.attachments),
        };
        self.reset_history_navigation();
        draft
    }

    fn record_submission_history(&mut self, submission: &ChatSubmission) {
        if submission
            .input
            .iter()
            .all(|input| matches!(input, ChatInputItem::Text(_)))
        {
            self.record_history(submission.display_text.clone());
        }
    }

    fn clear(&mut self) {
        self.textarea.clear();
        self.vim.reset_draft();
        self.slash_command_element = None;
        self.skill_bindings.clear();
        self.pending_pastes.clear();
        self.attachments.clear();
        self.reset_history_navigation();
    }

    pub(in crate::components) fn reconcile(&mut self, desired_command: Option<Range<usize>>) {
        self.pending_pastes.retain_present_in(&self.textarea);
        self.attachments.reconcile(&mut self.textarea);
        self.skill_bindings
            .retain(|(element_id, _)| self.textarea.has_element(*element_id));
        self.sync_slash_command_element(desired_command);
    }

    fn sync_slash_command_element(&mut self, desired: Option<Range<usize>>) {
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

    fn previous_history(&mut self) -> ChatInputOutcome {
        if self.history.is_empty() {
            return ChatInputOutcome::Consumed;
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
        ChatInputOutcome::Consumed
    }

    fn next_history(&mut self) -> ChatInputOutcome {
        let Some(index) = self.history_index else {
            return ChatInputOutcome::Consumed;
        };
        if index + 1 < self.history.len() {
            self.history_index = Some(index + 1);
            self.replace_with_history_entry(index + 1);
        } else {
            self.history_index = None;
            self.textarea.replace_text(&self.history_draft);
            self.history_draft.clear();
        }
        ChatInputOutcome::Consumed
    }

    fn replace_with_history_entry(&mut self, index: usize) {
        self.textarea.replace_text(&self.history[index]);
        self.vim.reset_draft();
        self.pending_pastes.clear();
        self.attachments.clear();
        self.slash_command_element = None;
        self.skill_bindings.clear();
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

    pub(in crate::components) fn apply_text_completion(
        &mut self,
        range: Range<usize>,
        replacement: &str,
    ) {
        self.textarea.replace_range(range, replacement);
    }

    pub(in crate::components) fn apply_element_completion(
        &mut self,
        range: Range<usize>,
        value: &str,
        skill: Option<SkillRef>,
    ) {
        self.textarea.replace_range(range, "");
        let element_id = self.textarea.insert_element(value);
        if let Some(skill) = skill {
            self.skill_bindings.push((element_id, skill));
        }
        if !self.textarea.text()[self.textarea.cursor()..]
            .chars()
            .next()
            .is_some_and(char::is_whitespace)
        {
            self.textarea.insert_text(" ");
        }
    }

    pub(in crate::components) fn draft_text(&self) -> &str {
        self.textarea.text()
    }

    pub(in crate::components) fn draft_cursor(&self) -> usize {
        self.textarea.cursor()
    }

    pub(in crate::components) fn slash_command_active(&self) -> bool {
        self.slash_command_element.is_some()
    }
}

fn into_command_invocation(
    mut submission: ChatSubmission,
    parsed: ParsedSlashCommand,
) -> Result<SlashCommandInvocation, ChatSubmission> {
    let command_prefix = format!("/{}", parsed.command.name);
    let Some(ChatInputItem::Text(first_text)) = submission.input.first_mut() else {
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

fn push_text_input(input: &mut Vec<ChatInputItem>, text: &mut String) {
    let text = std::mem::take(text);
    let text = text.trim();
    if !text.is_empty() {
        input.push(ChatInputItem::Text(text.to_owned()));
    }
}

fn is_newline_key(key: KeyEvent) -> bool {
    (matches!(key.code, KeyCode::Enter)
        && key
            .modifiers
            .intersects(KeyModifiers::SHIFT | KeyModifiers::ALT))
        || matches!(key.code, KeyCode::Char('j')) && key.modifiers == KeyModifiers::CONTROL
}
