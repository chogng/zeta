use crate::components::chat_input::ChatInput;
use crate::components::chat_input::ChatInputOutcome;
use crate::components::chat_input::ChatInputQueueOutcome;
use crate::components::chat_input::ChatSubmission;
use crate::components::chat_input::CompletionView;
use crate::components::chat_input::QueuedChatInput;
use crate::components::chat_input::SlashCommandInvocation;
use crate::components::steer::Steer;
use crate::components::steer::SteerId;
use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;

pub(crate) struct ChatComposerView<'a> {
    input: &'a ChatInput,
}

impl ChatComposerView<'_> {
    pub(super) fn input(&self) -> &str {
        self.input.text()
    }

    pub(super) fn input_cursor_width(&self) -> usize {
        self.input.cursor_display_width()
    }

    pub(super) fn input_cursor_line(&self) -> usize {
        self.input.cursor_line()
    }

    pub(super) fn input_prompt(&self) -> &'static str {
        self.input.prompt()
    }

    pub(super) fn input_desired_height(&self, available_width: u16) -> u16 {
        self.input.desired_height(available_width)
    }

    pub(super) fn input_completion(&self) -> Option<CompletionView<'_>> {
        self.input.completion()
    }
}

#[derive(Debug, Eq, PartialEq)]
pub(crate) enum ChatComposerOutcome {
    Command(SlashCommandInvocation),
    Consumed,
    SubmissionRejected(String),
    Queued(QueuedChatInput),
    Submit(ChatSubmission),
    Unhandled,
}

/// Coordinates ChatInput submission targets while leaving menus and feature flows to App.
#[derive(Debug)]
pub(crate) struct ChatComposer {
    steer: Steer,
}

impl ChatComposer {
    pub(crate) fn new() -> Self {
        Self {
            steer: Steer::default(),
        }
    }

    pub(crate) fn handle_key(
        &mut self,
        input: &mut ChatInput,
        key: KeyEvent,
    ) -> ChatComposerOutcome {
        self.handle_key_with_submission_target(input, key, SubmissionTarget::Start)
    }

    pub(crate) fn handle_active_turn_key(
        &mut self,
        input: &mut ChatInput,
        key: KeyEvent,
    ) -> ChatComposerOutcome {
        self.handle_key_with_submission_target(input, key, SubmissionTarget::Steer)
    }

    pub(crate) fn handle_queued_turn_key(
        &mut self,
        input: &mut ChatInput,
        key: KeyEvent,
    ) -> ChatComposerOutcome {
        self.handle_key_with_submission_target(input, key, SubmissionTarget::Queue)
    }

    fn handle_key_with_submission_target(
        &mut self,
        input: &mut ChatInput,
        key: KeyEvent,
        submission_target: SubmissionTarget,
    ) -> ChatComposerOutcome {
        if submission_target == SubmissionTarget::Queue
            && key.code == KeyCode::Enter
            && key.modifiers.is_empty()
            && input.completion().is_none()
            && input.accepts_submission_key()
        {
            return self.queue_current_input(input);
        }
        if submission_target == SubmissionTarget::Steer
            && key.code == KeyCode::Enter
            && key.modifiers.is_empty()
            && input.completion().is_none()
            && input.accepts_submission_key()
            && input.submission_contains_skill()
        {
            return ChatComposerOutcome::SubmissionRejected(
                "A running Turn cannot change its Skill; switch follow-up messages to Queue or wait for the next Turn"
                    .into(),
            );
        }
        map_chat_input_outcome(input.handle_key(key))
    }

    #[cfg(test)]
    pub(crate) fn insert_text(&mut self, input: &mut ChatInput, text: &str) {
        input.insert_text(text);
    }

    pub(crate) fn handle_paste(
        &mut self,
        input: &mut ChatInput,
        pasted: String,
    ) -> Result<(), String> {
        input.handle_paste(pasted)
    }

    pub(crate) fn attach_image_bytes(
        &mut self,
        input: &mut ChatInput,
        bytes: Vec<u8>,
    ) -> Result<(), String> {
        input.attach_image_bytes(bytes)
    }

    pub(crate) fn view<'a>(&self, input: &'a ChatInput) -> ChatComposerView<'a> {
        ChatComposerView { input }
    }

    pub(crate) fn activate_completion(
        &mut self,
        input: &mut ChatInput,
        index: usize,
    ) -> Option<ChatComposerOutcome> {
        input.activate_completion(index).map(map_chat_input_outcome)
    }

    pub(crate) fn begin_steer(&mut self, text: String) -> SteerId {
        self.steer.push(text)
    }

    pub(crate) fn finish_steer(&mut self, id: SteerId) -> bool {
        self.steer.remove(id)
    }

    pub(crate) fn clear_steers(&mut self) {
        self.steer.clear();
    }

    fn queue_current_input(&mut self, input: &mut ChatInput) -> ChatComposerOutcome {
        match input.queue_current() {
            ChatInputQueueOutcome::Command(command) => ChatComposerOutcome::Command(command),
            ChatInputQueueOutcome::Consumed => ChatComposerOutcome::Consumed,
            ChatInputQueueOutcome::Queued(input) => ChatComposerOutcome::Queued(input),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SubmissionTarget {
    Start,
    Queue,
    Steer,
}

fn map_chat_input_outcome(outcome: ChatInputOutcome) -> ChatComposerOutcome {
    match outcome {
        ChatInputOutcome::Command(command) => ChatComposerOutcome::Command(command),
        ChatInputOutcome::Consumed => ChatComposerOutcome::Consumed,
        ChatInputOutcome::Submit(prompt) => ChatComposerOutcome::Submit(prompt),
        ChatInputOutcome::Unhandled => ChatComposerOutcome::Unhandled,
    }
}

#[cfg(test)]
#[path = "state_tests.rs"]
mod tests;
