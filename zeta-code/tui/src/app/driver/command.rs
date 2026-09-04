use super::AppDriver;
use super::CommandEffect;
use crate::app::AppCommand;
use crate::app::Status;
use crate::app::completion::Completion;
use crate::app::completion::finish_product_command_request;
use crate::app::dispatch::execute_product_command;
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
use crate::thread::Command as ThreadCommand;
use crate::thread::CommandRequest as ThreadCommandRequest;
use crate::thread::Event as ThreadEvent;
use std::time::Instant;
use zeta_app_server_protocol::protocol::session::ThreadSnapshotHistory;

impl AppDriver {
    pub(in crate::app) fn execute(&mut self, command: AppCommand) -> CommandEffect {
        let request_key = request_key(&command);
        match command {
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
            AppCommand::Thread(ThreadCommand::ExecuteProductCommand(invocation)) => {
                if self.requests.is_idle(request_key) {
                    let mut client = self.client.clone();
                    let conversation = self.conversation.clone();
                    let subscription = self.thread_subscription.clone();
                    self.requests.spawn(
                        request_key,
                        "zeta-tui-product-command",
                        move || {
                            Completion::ProductCommand(
                                execute_product_command(conversation, &mut client, invocation)
                                    .and_then(|output| {
                                        finish_product_command_request(
                                            &mut client,
                                            subscription,
                                            output,
                                        )
                                    }),
                            )
                        },
                        &mut self.app,
                    );
                }
            }
            AppCommand::Quit => return CommandEffect::Quit,
            AppCommand::Thread(ThreadCommand::Interrupt) => {
                if let Some(turn_id) = self.app.active_turn().cloned()
                    && !matches!(self.app.status(), Status::Error)
                {
                    if self.requests.is_idle(request_key) {
                        let request = ThreadCommandRequest::Interrupt {
                            client: self.client.clone(),
                            scope: self.thread_request_scope(),
                            turn_id,
                            history: self.thread_subscription.history(),
                        };
                        let name = request.name();
                        self.requests.spawn(
                            request_key,
                            name,
                            move || Completion::Thread(request.execute()),
                            &mut self.app,
                        );
                    }
                } else if !matches!(self.app.status(), Status::Ready) {
                    self.app.update(ThreadEvent::InterruptFailed(
                        "the active turn is not available".into(),
                    ));
                }
            }
            AppCommand::Host(command) => {
                let operation = match command {
                    HostCommand::CopyLastResponse => HostOperation::CopyLastResponse(
                        self.app
                            .latest_agent_response()
                            .map(str::to_owned)
                            .ok_or_else(|| "there is no Zeta response to copy".to_owned()),
                    ),
                    HostCommand::ExportTranscript { requested_path } => {
                        HostOperation::ExportTranscript {
                            root: self.host_dir_root.clone(),
                            requested_path,
                            markdown: self.app.transcript_markdown(),
                        }
                    }
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
            AppCommand::Suspend => return CommandEffect::Suspend,
            AppCommand::Thread(ThreadCommand::LoadOlderHistory) => {
                if self.requests.is_idle(request_key)
                    && let Some(ThreadSnapshotHistory::Before { turn_id, .. }) =
                        self.thread_subscription.older_history()
                {
                    let request = ThreadCommandRequest::LoadOlderHistory {
                        client: self.client.clone(),
                        scope: self.thread_request_scope(),
                        before_turn_id: turn_id,
                    };
                    let name = request.name();
                    self.requests.spawn(
                        request_key,
                        name,
                        move || Completion::Thread(request.execute()),
                        &mut self.app,
                    );
                }
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
            AppCommand::Thread(ThreadCommand::OpenRewindPicker) => {
                if self.requests.is_idle(request_key) {
                    let request = ThreadCommandRequest::OpenRewindPicker {
                        client: self.client.clone(),
                        scope: self.thread_request_scope(),
                    };
                    let name = request.name();
                    self.requests.spawn(
                        request_key,
                        name,
                        move || Completion::Thread(request.execute()),
                        &mut self.app,
                    );
                }
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
            AppCommand::Thread(ThreadCommand::RewindToCheckpoint {
                before_turn_id,
                checkpoint_label,
            }) => {
                let command = format!("/rewind {before_turn_id}");
                self.app
                    .update(ThreadEvent::CommandStarted(command.clone()));
                if self.requests.is_idle(request_key) {
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
                                    .rewind_active_thread(
                                        &mut client,
                                        before_turn_id,
                                        &checkpoint_label,
                                    )
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
            AppCommand::Sessions(command) => {
                if let Some(command_line) = command.command_line() {
                    self.app.update(ThreadEvent::CommandStarted(command_line));
                }
                let request: SessionCommandRequest = sessions::prepare_command(
                    self.client.clone(),
                    &self.conversation,
                    &self.thread_subscription,
                    self.app.approval_mode(),
                    command,
                );
                let name = request.name();
                self.requests.spawn(
                    request_key,
                    name,
                    move || Completion::Sessions(request.execute()),
                    &mut self.app,
                );
            }
            AppCommand::Thread(ThreadCommand::ResolveRequest(response)) => {
                if self.requests.is_idle(request_key) {
                    let request = ThreadCommandRequest::ResolveRequest {
                        client: self.client.clone(),
                        scope: self.thread_request_scope(),
                        request: response.identity(),
                        response,
                        history: self.thread_subscription.history(),
                    };
                    let name = request.name();
                    self.requests.spawn(
                        request_key,
                        name,
                        move || Completion::Thread(request.execute()),
                        &mut self.app,
                    );
                }
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
            AppCommand::Thread(ThreadCommand::CycleNextApprovalMode) => {
                self.app.cycle_next_approval_mode(Instant::now());
            }
            AppCommand::Thread(ThreadCommand::SubmitTurn { submission }) => {
                if self.requests.is_idle(request_key) {
                    let request = ThreadCommandRequest::SubmitTurn {
                        client: self.client.clone(),
                        scope: self.thread_request_scope(),
                        submission,
                        approval_mode: self.app.approval_mode(),
                        history: self.thread_subscription.history(),
                    };
                    let name = request.name();
                    self.requests.spawn(
                        request_key,
                        name,
                        move || Completion::Thread(request.execute()),
                        &mut self.app,
                    );
                }
            }
            AppCommand::Thread(ThreadCommand::SubmitQueuedTurn {
                queue_id,
                submission,
            }) => {
                if self.requests.is_idle(request_key) {
                    let request = ThreadCommandRequest::SubmitQueuedTurn {
                        client: self.client.clone(),
                        scope: self.thread_request_scope(),
                        queue_id,
                        submission,
                        approval_mode: self.app.approval_mode(),
                        history: self.thread_subscription.history(),
                    };
                    let name = request.name();
                    self.requests.spawn(
                        request_key,
                        name,
                        move || Completion::Thread(request.execute()),
                        &mut self.app,
                    );
                }
            }
            AppCommand::Thread(ThreadCommand::SteerTurn {
                source,
                steer_id,
                submission,
            }) => {
                if self.requests.is_idle(request_key) {
                    if matches!(self.app.status(), Status::Working)
                        && !self.app.steers_active_turn()
                    {
                        self.queued_commands.push_front(
                            ThreadCommand::SteerTurn {
                                source,
                                steer_id,
                                submission,
                            }
                            .into(),
                        );
                    } else if let Some(turn_id) = self.app.active_turn().cloned() {
                        let request = ThreadCommandRequest::SteerTurn {
                            client: self.client.clone(),
                            scope: self.thread_request_scope(),
                            turn_id,
                            source,
                            steer_id,
                            submission,
                            history: self.thread_subscription.history(),
                        };
                        let name = request.name();
                        self.requests.spawn(
                            request_key,
                            name,
                            move || Completion::Thread(request.execute()),
                            &mut self.app,
                        );
                    } else {
                        self.app.update(ThreadEvent::SteerSubmissionFailed {
                            source,
                            steer_id,
                            error: "the active Turn is no longer available".into(),
                        });
                    }
                }
            }
        }
        CommandEffect::None
    }
}
