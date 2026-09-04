use super::AppDriver;
use super::CommandEffect;
use crate::app::AppCommand;
use crate::app::completion::Completion;
use crate::app::completion::finish_product_command_request;
use crate::app::dispatch::execute_product_command;
use crate::app::requests::RequestKey;
use crate::app::requests::request_key;
use crate::config;
use crate::connectors;
use crate::dirs;
use crate::host::Command as HostCommand;
use crate::host::Operation as HostOperation;
use crate::keymap;
use crate::mcp;
use crate::sessions;
use crate::sessions::CommandRequest as SessionCommandRequest;
use crate::sessions::SessionCompletion;
use crate::sessions::finish_conversation_request;
use crate::skills;
use crate::status as status_line;
use crate::theme as theme_feature;
use crate::thread;
use crate::thread::Command as ThreadCommand;
use crate::thread::CommandPreparation as ThreadCommandPreparation;
use crate::thread::Event as ThreadEvent;
use std::time::Instant;

impl AppDriver {
    pub(in crate::app) fn execute(&mut self, command: AppCommand) -> CommandEffect {
        let request_key = request_key(&command);
        match command {
            AppCommand::Config(command) => {
                let name = command.request_name();
                let mut client = self.client.clone();
                self.requests.spawn_presentation(
                    request_key,
                    name,
                    move || config::execute(&mut client, command),
                    &mut self.app,
                );
            }
            AppCommand::Connectors(command) => {
                let name = command.request_name();
                let mut client = self.client.clone();
                self.requests.spawn_presentation(
                    request_key,
                    name,
                    move || connectors::execute(&mut client, command),
                    &mut self.app,
                );
            }
            AppCommand::Dirs(command) => {
                let name = command.request_name();
                let mut client = self.client.clone();
                let session_id = self.conversation.session_id().clone();
                self.requests.spawn_presentation(
                    request_key,
                    name,
                    move || dirs::execute(&mut client, &session_id, command),
                    &mut self.app,
                );
            }
            AppCommand::Host(command) => self.execute_host_command(request_key, command),
            AppCommand::Keymap(command) => {
                let name = command.request_name();
                let mut client = self.client.clone();
                self.requests.spawn_presentation(
                    request_key,
                    name,
                    move || keymap::execute(&mut client, command),
                    &mut self.app,
                );
            }
            AppCommand::Mcp(command) => {
                let name = command.request_name();
                let mut client = self.client.clone();
                self.requests.spawn_presentation(
                    request_key,
                    name,
                    move || mcp::execute(&mut client, command),
                    &mut self.app,
                );
            }
            AppCommand::Models(command) => {
                let command_line = command.command_line();
                self.app
                    .update(ThreadEvent::CommandStarted(command_line.clone()));
                let name = command.request_name();
                let mut client = self.client.clone();
                self.requests.spawn(
                    request_key,
                    name,
                    move || Completion::PreferredModelUpdated {
                        command: command_line,
                        result: crate::models::execute(&mut client, command),
                    },
                    &mut self.app,
                );
            }
            AppCommand::Sessions(command) => self.execute_session_command(request_key, command),
            AppCommand::Skills(command) => {
                let name = command.request_name();
                let mut client = self.client.clone();
                let session_id = self.conversation.session_id().clone();
                self.requests.spawn_presentation(
                    request_key,
                    name,
                    move || skills::execute(&mut client, &session_id, command),
                    &mut self.app,
                );
            }
            AppCommand::Status(command) => {
                let name = command.request_name();
                let mut client = self.client.clone();
                self.requests.spawn_presentation(
                    request_key,
                    name,
                    move || status_line::execute(&mut client, command),
                    &mut self.app,
                );
            }
            AppCommand::Theme(command) => {
                if let Some(command_line) = command.command_line() {
                    self.app.update(ThreadEvent::CommandStarted(command_line));
                }
                let name = command.request_name();
                let mut client = self.client.clone();
                let theme_resource = self.theme_resource.clone();
                self.requests.spawn(
                    request_key,
                    name,
                    move || {
                        Completion::Theme(theme_feature::execute(
                            &mut client,
                            &theme_resource,
                            command,
                        ))
                    },
                    &mut self.app,
                );
            }
            AppCommand::Thread(command) => self.execute_thread_command(request_key, command),
            AppCommand::Quit => return CommandEffect::Quit,
            AppCommand::Suspend => return CommandEffect::Suspend,
        }
        CommandEffect::None
    }

    fn execute_host_command(&mut self, request_key: Option<RequestKey>, command: HostCommand) {
        let operation = match command {
            HostCommand::CopyLastResponse => HostOperation::CopyLastResponse(
                self.app
                    .latest_agent_response()
                    .map(str::to_owned)
                    .ok_or_else(|| "there is no Zeta response to copy".to_owned()),
            ),
            HostCommand::ExportTranscript { requested_path } => HostOperation::ExportTranscript {
                root: self.host_dir_root.clone(),
                requested_path,
                markdown: self.app.transcript_markdown(),
            },
            HostCommand::ReadClipboardImage => HostOperation::ReadClipboardImage,
            HostCommand::RefreshClipboardImageAvailability => {
                HostOperation::RefreshClipboardImageAvailability
            }
        };
        let name = operation.name();
        self.requests.spawn_presentation(
            request_key,
            name,
            move || Ok(operation.execute()),
            &mut self.app,
        );
    }

    fn execute_session_command(
        &mut self,
        request_key: Option<RequestKey>,
        command: sessions::Command,
    ) {
        if let Some(command_line) = command.command_line() {
            self.app.update(ThreadEvent::CommandStarted(command_line));
        }
        let request: SessionCommandRequest =
            sessions::prepare_command(self.app.approval_mode(), command);
        let name = request.name();
        let client = self.client.clone();
        let conversation = self.conversation.clone();
        let subscription = self.thread_subscription.clone();
        self.requests.spawn(
            request_key,
            name,
            move || Completion::Sessions(request.execute(client, conversation, subscription)),
            &mut self.app,
        );
    }

    fn execute_thread_command(&mut self, request_key: Option<RequestKey>, command: ThreadCommand) {
        let preparation = thread::prepare_command(
            self.thread_subscription.older_history(),
            self.app.thread_command_state(),
            command,
        );
        match preparation {
            ThreadCommandPreparation::ExecuteProductCommand(invocation) => {
                self.execute_product_command(request_key, invocation);
            }
            ThreadCommandPreparation::RewindToCheckpoint {
                before_turn_id,
                checkpoint_label,
            } => self.execute_rewind(request_key, before_turn_id, checkpoint_label),
            ThreadCommandPreparation::CycleNextApprovalMode => {
                self.app.cycle_next_approval_mode(Instant::now());
            }
            ThreadCommandPreparation::Request(request) => {
                let name = request.name();
                let client = self.client.clone();
                let scope = self.thread_request_scope();
                let history = self.thread_subscription.history();
                self.requests.spawn(
                    request_key,
                    name,
                    move || Completion::Thread(request.execute(client, scope, history)),
                    &mut self.app,
                );
            }
            ThreadCommandPreparation::Present(event) => self.app.update(event),
            ThreadCommandPreparation::Requeue(command) => {
                self.queued_commands.push_front(command.into());
            }
            ThreadCommandPreparation::None => {}
        }
    }

    fn execute_product_command(
        &mut self,
        request_key: Option<RequestKey>,
        invocation: crate::thread::composer::SlashCommandInvocation,
    ) {
        let mut client = self.client.clone();
        let conversation = self.conversation.clone();
        let subscription = self.thread_subscription.clone();
        self.requests.spawn(
            request_key,
            "zeta-tui-product-command",
            move || {
                Completion::ProductCommand(
                    execute_product_command(conversation, &mut client, invocation).and_then(
                        |output| finish_product_command_request(&mut client, subscription, output),
                    ),
                )
            },
            &mut self.app,
        );
    }

    fn execute_rewind(
        &mut self,
        request_key: Option<RequestKey>,
        before_turn_id: zeta_protocol::TurnId,
        checkpoint_label: String,
    ) {
        let command = format!("/rewind {before_turn_id}");
        self.app
            .update(ThreadEvent::CommandStarted(command.clone()));
        let mut client = self.client.clone();
        let mut conversation = self.conversation.clone();
        let subscription = self.thread_subscription.clone();
        self.requests.spawn(
            request_key,
            "zeta-tui-rewind-thread",
            move || {
                Completion::Sessions(SessionCompletion::Changed {
                    command,
                    result: conversation
                        .rewind_active_thread(&mut client, before_turn_id, &checkpoint_label)
                        .map_err(|error| error.to_string())
                        .and_then(|change| {
                            finish_conversation_request(
                                &mut client,
                                conversation,
                                subscription,
                                change,
                            )
                        }),
                })
            },
            &mut self.app,
        );
    }
}
