use super::attachments::Attachments;
use super::attachments::ImagePasteOutcome;
use super::editor::TextArea;
use super::editor::TextAreaOutcome;
use super::editor::TextElementId;
use super::mentions::MentionPopupView;
use super::mentions::Mentions;
use super::pending_pastes::PendingPastes;
use super::slash_command_popup::SlashCommandPopup;
use super::slash_command_popup::SlashPopupView;
use super::slash_commands::SlashCommandItem;
use super::slash_commands::SlashCommandRegistry;
use super::slash_input::ParsedSlashCommand;
use super::slash_input::SlashInput;
use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use zeta_file_search::PathSearchSnapshot;

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
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ComposerSubmission {
    pub(crate) display_text: String,
    pub(crate) input: Vec<ComposerInput>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct SlashCommandInvocation {
    pub(crate) command: SlashCommandItem,
    pub(crate) display_arguments: String,
    pub(crate) arguments: Vec<ComposerInput>,
}

impl SlashCommandInvocation {
    pub(crate) fn into_forwarded_submission(mut self) -> ComposerSubmission {
        let command_text = format!("/{}", self.command.command());
        let display_text = if self.display_arguments.is_empty() {
            command_text.clone()
        } else {
            format!("{command_text} {}", self.display_arguments)
        };

        match self.arguments.first_mut() {
            Some(ComposerInput::Text(text)) => {
                *text = format!("{command_text} {text}");
            }
            Some(ComposerInput::Image { .. }) | None => {
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
/// Slash parsing belongs to [`SlashInput`]; this component applies its completion plans, owns popup
/// key routing, and turns parsed submissions into command invocations. The text area remains
/// responsible only for editing state and keymap behavior.
#[derive(Debug)]
pub(crate) struct ChatComposer {
    textarea: TextArea,
    slash_commands: SlashCommandRegistry,
    slash_popup: SlashCommandPopup,
    slash_command_element: Option<TextElementId>,
    mentions: Mentions,
    pending_pastes: PendingPastes,
    attachments: Attachments,
}

impl ChatComposer {
    #[cfg(test)]
    pub(crate) fn new() -> Self {
        Self::with_slash_commands(SlashCommandRegistry::default())
    }

    pub(crate) fn with_slash_commands(slash_commands: SlashCommandRegistry) -> Self {
        Self {
            textarea: TextArea::new(),
            slash_commands,
            slash_popup: SlashCommandPopup::default(),
            slash_command_element: None,
            mentions: Mentions::default(),
            pending_pastes: PendingPastes::default(),
            attachments: Attachments::default(),
        }
    }

    pub(crate) fn handle_key(&mut self, key: KeyEvent) -> ComposerOutcome {
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
                KeyCode::Enter if self.mentions.complete_selected(&mut self.textarea) => {
                    self.sync_after_text_change();
                    return ComposerOutcome::Consumed;
                }
                _ => {}
            }
        }

        if self.slash_popup.view().is_some() {
            match key.code {
                KeyCode::Esc => {
                    self.slash_popup.dismiss();
                    return ComposerOutcome::Consumed;
                }
                KeyCode::Up => {
                    self.slash_popup.select_previous();
                    return ComposerOutcome::Consumed;
                }
                KeyCode::Down => {
                    self.slash_popup.select_next();
                    return ComposerOutcome::Consumed;
                }
                KeyCode::Tab => {
                    if let Some(command) = self.slash_popup.selected_command() {
                        self.complete_slash_command(&command);
                    }
                    return ComposerOutcome::Consumed;
                }
                KeyCode::Enter => {
                    if let Some(command) = self.slash_popup.selected_command() {
                        self.complete_slash_command(&command);
                        return self.submit();
                    }
                }
                _ => {}
            }
        }

        if key.code == KeyCode::Enter {
            return self.submit();
        }

        match self.textarea.handle_key(key) {
            TextAreaOutcome::Consumed => {
                self.sync_after_text_change();
                ComposerOutcome::Consumed
            }
            TextAreaOutcome::Unhandled => ComposerOutcome::Unhandled,
        }
    }

    #[cfg(test)]
    pub(crate) fn insert_text(&mut self, text: &str) {
        self.textarea.insert_text(text);
        self.sync_after_text_change();
    }

    pub(crate) fn handle_paste(&mut self, pasted: String) -> Result<(), String> {
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

    pub(crate) fn slash_popup(&self) -> Option<SlashPopupView<'_>> {
        self.slash_popup.view()
    }

    pub(crate) fn mention_popup(&self) -> Option<MentionPopupView<'_>> {
        self.mentions.view()
    }

    pub(crate) fn mention_query(&self) -> Option<&str> {
        self.mentions.query()
    }

    pub(crate) fn apply_file_search_snapshot(&mut self, snapshot: PathSearchSnapshot) {
        self.mentions.apply_search_snapshot(snapshot);
    }

    pub(crate) fn activate_slash_command(&mut self, index: usize) -> Option<ComposerOutcome> {
        let command = self.slash_popup.command_at(index)?;
        self.complete_slash_command(&command);
        Some(self.submit())
    }

    pub(crate) fn activate_mention(&mut self, index: usize) -> bool {
        let completed = self.mentions.complete_at(&mut self.textarea, index);
        if completed {
            self.sync_after_text_change();
        }
        completed
    }

    fn submit(&mut self) -> ComposerOutcome {
        let Some(submission) = self.prepare_submission() else {
            return ComposerOutcome::Consumed;
        };
        let command = SlashInput::for_submission(&submission.display_text, &self.slash_commands)
            .submission_command();
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
            }
            cursor = range.end;
        }
        text.push_str(&raw_text[cursor..]);
        push_text_input(&mut input, &mut text);

        (!input.is_empty()).then_some(ComposerSubmission {
            display_text,
            input,
        })
    }

    fn clear(&mut self) {
        self.textarea.clear();
        self.slash_popup.clear();
        self.slash_command_element = None;
        self.mentions.clear();
        self.pending_pastes.clear();
        self.attachments.clear();
    }

    fn complete_slash_command(&mut self, command: &SlashCommandItem) -> bool {
        let Some(completion) = SlashInput::at_cursor(
            self.textarea.text(),
            self.textarea.cursor(),
            &self.slash_commands,
        )
        .completion(command) else {
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
        if self.slash_command_element.is_some() {
            self.slash_popup.clear();
        } else {
            self.slash_popup.sync_input(
                self.textarea.text(),
                self.textarea.cursor(),
                &self.slash_commands,
            );
        }
    }

    fn sync_slash_command_element(&mut self) {
        let desired = SlashInput::at_cursor(
            self.textarea.text(),
            self.textarea.cursor(),
            &self.slash_commands,
        )
        .command_element_range();

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
}

fn into_command_invocation(
    mut submission: ComposerSubmission,
    parsed: ParsedSlashCommand,
) -> Result<SlashCommandInvocation, ComposerSubmission> {
    let command_prefix = format!("/{}", parsed.command.command());
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

#[cfg(test)]
#[path = "state_tests.rs"]
mod tests;
