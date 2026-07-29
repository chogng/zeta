use crate::chatwidget::ChatWidget;
use crate::chatwidget::ChatWidgetOutcome;
pub(crate) use crate::chatwidget::Message;
pub(crate) use crate::chatwidget::MessageRole;
use crate::file_search::FileSearchManager;
use crate::selection_views::{SkillSelectionAction, SkillSelectionView};
use crate::status_line::StatusLineModel;
use crate::toppane::ComposerSubmission;
use crate::toppane::MentionPopupView;
use crate::toppane::SelectionItemId;
use crate::toppane::SelectionViewModel;
use crate::toppane::SelectionViewState;
use crate::toppane::SlashCommand;
use crate::toppane::SlashCommandInvocation;
use crate::toppane::SlashCommandItem;
use crate::toppane::SlashCommandRegistry;
use crate::toppane::SlashPopupView;
use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use crossterm::event::KeyModifiers;
use std::collections::BTreeMap;
use std::path::Path;
use zeta_app_server_protocol::protocol::config::ConfigReadResult;
use zeta_app_server_protocol::protocol::skills::SkillEnablementDto;
use zeta_protocol::{SkillId, Thread, ThreadItem};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum Action {
    Command(SlashCommandInvocation),
    Quit,
    Interrupt,
    PasteImage,
    SetSkillEnablement {
        skill_id: SkillId,
        enablement: SkillEnablementDto,
    },
    Submit(ComposerSubmission),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum Status {
    Ready,
    Working,
    WaitingForApproval,
    WaitingForUserInput,
    WaitingForCapability,
    Cancelling,
    Error,
}

#[derive(Debug)]
pub(crate) struct App {
    chat_widget: ChatWidget,
    file_search: Option<FileSearchManager>,
    selection_actions: Vec<SelectionActions>,
    status: Status,
    status_line: StatusLineModel,
}

#[derive(Debug)]
enum SelectionActions {
    ReadOnly,
    Skills(BTreeMap<SelectionItemId, SkillSelectionAction>),
}

impl App {
    #[cfg(test)]
    pub(crate) fn new() -> Self {
        Self {
            chat_widget: ChatWidget::new(),
            file_search: None,
            selection_actions: Vec::new(),
            status: Status::Ready,
            status_line: StatusLineModel::for_workspace(Path::new(".")),
        }
    }

    #[cfg(test)]
    pub(crate) fn for_workspace(workspace_root: &Path) -> Self {
        Self::for_workspace_with_slash_commands(workspace_root, SlashCommandRegistry::default())
    }

    pub(crate) fn for_workspace_with_slash_commands(
        workspace_root: &Path,
        slash_commands: SlashCommandRegistry,
    ) -> Self {
        Self {
            chat_widget: ChatWidget::with_slash_commands(slash_commands),
            file_search: Some(FileSearchManager::new(workspace_root.to_path_buf())),
            selection_actions: Vec::new(),
            status: Status::Ready,
            status_line: StatusLineModel::for_workspace(workspace_root),
        }
    }

    pub(crate) fn handle_key(&mut self, key: KeyEvent) -> Option<Action> {
        if !self.accepts_input() {
            return self.handle_global_key(key);
        }

        let outcome = self.chat_widget.handle_key(key);
        self.sync_file_search_query();
        if matches!(outcome, ChatWidgetOutcome::Unhandled) {
            return self.handle_global_key(key);
        }
        self.handle_chat_widget_outcome(outcome)
    }

    fn handle_chat_widget_outcome(&mut self, outcome: ChatWidgetOutcome) -> Option<Action> {
        match outcome {
            ChatWidgetOutcome::ActivateSelectionItem(item_id) => {
                self.activate_selection_item(&item_id)
            }
            ChatWidgetOutcome::Command(command) => self.handle_slash_command(command),
            ChatWidgetOutcome::Submit(submission) => {
                self.status = Status::Working;
                Some(Action::Submit(submission))
            }
            ChatWidgetOutcome::Consumed => None,
            ChatWidgetOutcome::Unhandled => None,
            ChatWidgetOutcome::ViewDismissed => {
                self.selection_actions.pop();
                None
            }
        }
    }

    fn activate_selection_item(&self, item_id: &SelectionItemId) -> Option<Action> {
        let SelectionActions::Skills(actions) = self.selection_actions.last()? else {
            return None;
        };
        match actions.get(item_id)? {
            SkillSelectionAction::SetEnablement {
                skill_id,
                enablement,
            } => Some(Action::SetSkillEnablement {
                skill_id: skill_id.clone(),
                enablement: *enablement,
            }),
        }
    }

    pub(crate) fn activate_slash_command(&mut self, index: usize) -> Option<Action> {
        if !self.accepts_input() {
            return None;
        }
        let outcome = self.chat_widget.activate_slash_command(index)?;
        self.handle_chat_widget_outcome(outcome)
    }

    #[cfg(test)]
    pub(crate) fn insert_text(&mut self, text: &str) {
        if self.accepts_input() {
            self.chat_widget.insert_text(text);
            self.sync_file_search_query();
        }
    }

    pub(crate) fn handle_paste(&mut self, pasted: String) {
        if self.accepts_input()
            && let Err(error) = self.chat_widget.handle_paste(pasted)
        {
            self.chat_widget.push_message(MessageRole::Error, error);
        }
        self.sync_file_search_query();
    }

    pub(crate) fn attach_image_bytes(&mut self, bytes: Vec<u8>) {
        if self.accepts_input()
            && let Err(error) = self.chat_widget.attach_image_bytes(bytes)
        {
            self.chat_widget.push_message(MessageRole::Error, error);
        }
        self.sync_file_search_query();
    }

    pub(crate) fn record_clipboard_error(&mut self, error: String) {
        if self.accepts_input() {
            self.chat_widget.push_message(
                MessageRole::Error,
                format!("could not paste clipboard image: {error}"),
            );
        }
    }

    pub(crate) fn input(&self) -> &str {
        self.chat_widget.draft()
    }

    pub(crate) fn input_cursor_width(&self) -> usize {
        self.chat_widget.draft_cursor_width()
    }

    pub(crate) fn slash_popup(&self) -> Option<SlashPopupView<'_>> {
        self.chat_widget.slash_popup()
    }

    pub(crate) fn mention_popup(&self) -> Option<MentionPopupView<'_>> {
        self.chat_widget.mention_popup()
    }

    pub(crate) fn show_selection_view(&mut self, model: SelectionViewModel) {
        self.chat_widget.show_selection_view(model);
        self.selection_actions.push(SelectionActions::ReadOnly);
    }

    pub(crate) fn show_skills_view(&mut self, view: SkillSelectionView) {
        self.chat_widget.show_selection_view(view.model);
        self.selection_actions
            .push(SelectionActions::Skills(view.actions));
    }

    pub(crate) fn replace_skills_view(&mut self, view: SkillSelectionView) {
        self.chat_widget.replace_selection_view(view.model);
        match self.selection_actions.last_mut() {
            Some(actions) => *actions = SelectionActions::Skills(view.actions),
            None => self
                .selection_actions
                .push(SelectionActions::Skills(view.actions)),
        }
    }

    pub(crate) fn skills_view_is_active(&self) -> bool {
        matches!(
            self.selection_actions.last(),
            Some(SelectionActions::Skills(_))
        )
    }

    pub(crate) fn selection_view(&self) -> Option<&SelectionViewState> {
        self.chat_widget.selection_view()
    }

    pub(crate) fn activate_mention(&mut self, index: usize) -> bool {
        let activated = self.accepts_input() && self.chat_widget.activate_mention(index);
        self.sync_file_search_query();
        activated
    }

    pub(crate) fn poll_background_events(&mut self) {
        let snapshots = self
            .file_search
            .as_mut()
            .map(FileSearchManager::poll)
            .unwrap_or_default();
        for snapshot in snapshots {
            self.chat_widget.apply_file_search_snapshot(snapshot);
        }
    }

    pub(crate) fn messages(&self) -> &[Message] {
        self.chat_widget.messages()
    }

    pub(crate) fn status(&self) -> &Status {
        &self.status
    }

    pub(crate) fn status_line(&self) -> &StatusLineModel {
        &self.status_line
    }

    pub(crate) fn apply_config_snapshot(&mut self, config: &ConfigReadResult) {
        self.status_line.apply_config(config);
    }

    pub(crate) fn accepts_input(&self) -> bool {
        matches!(&self.status, Status::Ready | Status::Error)
    }

    pub(crate) fn record_response(&mut self, response: String) {
        self.chat_widget.push_message(MessageRole::Agent, response);
        self.status = Status::Ready;
    }

    pub(crate) fn record_notice(&mut self, notice: impl Into<String>) {
        self.chat_widget
            .push_message(MessageRole::Notice, notice.into());
        self.status = Status::Ready;
    }

    pub(crate) fn load_thread(&mut self, thread: &Thread) {
        self.chat_widget.clear_messages();
        for item in thread.turns.iter().flat_map(|turn| &turn.items) {
            match item {
                ThreadItem::UserMessage { text, .. } => {
                    self.chat_widget
                        .push_message(MessageRole::User, text.clone());
                }
                ThreadItem::UserImage { .. } => {
                    self.chat_widget
                        .push_message(MessageRole::User, "[Image]".into());
                }
                ThreadItem::AgentMessage { text, .. } => {
                    self.chat_widget
                        .push_message(MessageRole::Agent, text.clone());
                }
                ThreadItem::Reasoning { .. }
                | ThreadItem::Plan { .. }
                | ThreadItem::ToolCall { .. }
                | ThreadItem::ToolResult { .. } => {}
            }
        }
        self.status = Status::Ready;
    }

    pub(crate) fn clear_messages(&mut self) {
        self.chat_widget.clear_messages();
        self.status = Status::Ready;
    }

    pub(crate) fn record_interrupted(&mut self) {
        self.chat_widget
            .push_message(MessageRole::Notice, "turn interrupted".into());
        self.status = Status::Ready;
    }

    pub(crate) fn record_interrupt_failure(&mut self, error: String) {
        self.chat_widget.push_message(
            MessageRole::Error,
            format!("could not interrupt turn: {error}"),
        );
        self.status = Status::Working;
    }

    pub(crate) fn record_working(&mut self) {
        self.status = Status::Working;
    }

    pub(crate) fn record_cancelling(&mut self) {
        self.status = Status::Cancelling;
    }

    pub(crate) fn wait_for_approval(&mut self) {
        self.status = Status::WaitingForApproval;
    }

    pub(crate) fn wait_for_user_input(&mut self) {
        self.status = Status::WaitingForUserInput;
    }

    pub(crate) fn wait_for_capability(&mut self) {
        self.status = Status::WaitingForCapability;
    }

    pub(crate) fn record_error(&mut self, error: String) {
        self.chat_widget.push_message(MessageRole::Error, error);
        self.status = Status::Error;
    }

    fn handle_global_key(&mut self, key: KeyEvent) -> Option<Action> {
        if self.accepts_input()
            && key.modifiers.contains(KeyModifiers::CONTROL)
            && matches!(key.code, KeyCode::Char(character) if character.eq_ignore_ascii_case(&'v'))
        {
            return Some(Action::PasteImage);
        }
        if key.modifiers.contains(KeyModifiers::CONTROL) {
            return match key.code {
                KeyCode::Char('c') => self.quit_or_interrupt(),
                KeyCode::Char('d') if self.input().is_empty() => self.quit_or_interrupt(),
                _ => None,
            };
        }
        if key.code == KeyCode::Esc {
            return self.quit_or_interrupt();
        }
        None
    }

    fn handle_slash_command(&mut self, invocation: SlashCommandInvocation) -> Option<Action> {
        match &invocation.command {
            SlashCommandItem::Builtin(SlashCommand::Quit | SlashCommand::Exit)
                if invocation.arguments.is_empty() =>
            {
                Some(Action::Quit)
            }
            SlashCommandItem::Dynamic(_) => {
                let submission = invocation.into_forwarded_submission();
                self.chat_widget
                    .push_message(MessageRole::User, submission.display_text.clone());
                self.status = Status::Working;
                Some(Action::Submit(submission))
            }
            SlashCommandItem::Builtin(_) => Some(Action::Command(invocation)),
        }
    }

    fn quit_or_interrupt(&mut self) -> Option<Action> {
        match &self.status {
            Status::Working
            | Status::WaitingForApproval
            | Status::WaitingForUserInput
            | Status::WaitingForCapability => {
                self.status = Status::Cancelling;
                Some(Action::Interrupt)
            }
            Status::Cancelling => None,
            Status::Ready | Status::Error => Some(Action::Quit),
        }
    }

    fn sync_file_search_query(&mut self) {
        let query = self.chat_widget.mention_query().map(str::to_owned);
        let Some(file_search) = &mut self.file_search else {
            return;
        };
        if let Some(query) = query {
            file_search.update_query(&query);
        } else {
            file_search.stop();
        }
    }
}

#[cfg(test)]
#[path = "app_tests.rs"]
mod tests;
