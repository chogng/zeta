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
                        let command_line = command.command_line();
                        match (
                            command_line,
                            theme_feature::execute(&mut client, &theme_resource, command),
                        ) {
                            (Some(command), Err(error)) => {
                                Completion::Presentation(Ok(ThreadEvent::CommandFailed {
                                    command,
                                    error,
                                }
                                .into()))
                            }
                            (_, result) => Completion::Theme(result),
                        }
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

    fn execute_memory(&mut self, key: Option<RequestKey>, arguments: &str) {
        let mut parts = arguments.trim().splitn(2, char::is_whitespace);
        let action = parts.next().unwrap_or("read");
        let path = parts
            .next()
            .map(str::trim)
            .filter(|value| !value.is_empty());
        match action {
            "start" if path.is_none() => {
                if self.memory.as_ref().is_some_and(|recording| {
                    recording.report().is_ok_and(|report| {
                        report.status == zeta_memory_diagnostics::MemoryStatus::Recording
                    })
                }) {
                    self.app.update(crate::host::Event::OperationCompleted(Ok("Memory diagnostics are already recording. Use /memory read or /memory stop.".into())));
                    return;
                }
                let client = self.client.clone();
                let id = crate::client::new_command_id("memory").to_string();
                let objects = std::sync::Arc::clone(&self.memory_objects);
                self.requests.spawn(
                    key,
                    "zeta-memory-start",
                    move || {
                        Completion::Memory(
                            zeta_app_server_client::MemoryRecording::start(
                                client,
                                zeta_memory_diagnostics::MemoryProduct::Tui,
                                id,
                                move || {
                                    vec![zeta_memory_diagnostics::MemoryMetric {
                                        kind: zeta_memory_diagnostics::MemoryMetricKind::UiObjects,
                                        value: Some(
                                            objects.load(std::sync::atomic::Ordering::Relaxed)
                                                as u64,
                                        ),
                                        unavailable: None,
                                    }]
                                },
                            )
                            .map(Box::new),
                        )
                    },
                    &mut self.app,
                );
            }
            "stop" if path.is_none() => {
                let Some(mut recording) = self.memory.take() else {
                    self.app.update(crate::host::Event::OperationCompleted(Ok(
                        "No memory diagnostic has been started.".into(),
                    )));
                    return;
                };
                self.requests.spawn(
                    key,
                    "zeta-memory-stop",
                    move || Completion::Memory(recording.stop().map(|_| recording)),
                    &mut self.app,
                );
            }
            "read" | "" if path.is_none() => {
                let result = self
                    .memory
                    .as_ref()
                    .ok_or_else(|| "Use /memory start to begin a diagnostic.".to_owned())
                    .and_then(|recording| recording.report())
                    .map(|report| crate::memory::describe(&report));
                self.app
                    .update(crate::host::Event::OperationCompleted(result));
            }
            "export" if path.is_some() => {
                let Some(recording) = &self.memory else {
                    self.app.update(crate::host::Event::OperationCompleted(Err(
                        "No memory diagnostic has been started.".into(),
                    )));
                    return;
                };
                let id = recording.session_id().to_owned();
                let path = self.host_dir_root.join(path.unwrap());
                let mut client = self.client.clone();
                self.requests.spawn_presentation(
                    key,
                    "zeta-memory-export",
                    move || {
                        let result = (|| {
                            use std::io::Write;
                            let bytes = client.export_memory_bytes(id)?;
                            let mut file = std::fs::OpenOptions::new()
                                .write(true)
                                .create_new(true)
                                .open(&path)
                                .map_err(|error| error.to_string())?;
                            file.write_all(&bytes).map_err(|error| error.to_string())?;
                            Ok(format!("Memory report exported to {}", path.display()))
                        })();
                        Ok(crate::host::Event::OperationCompleted(result))
                    },
                    &mut self.app,
                );
            }
            _ => self.app.update(crate::host::Event::OperationCompleted(Err(
                "Usage: /memory start | read | stop | export <new-file.json>".into(),
            ))),
        }
    }

    fn execute_product_command(
        &mut self,
        request_key: Option<RequestKey>,
        invocation: crate::thread::composer::SlashCommandInvocation,
    ) {
        if invocation.command.name == "memory" {
            self.execute_memory(request_key, &invocation.display_arguments);
            return;
        }
        let mut client = self.client.clone();
        let conversation = self.conversation.clone();
        let subscription = self.thread_subscription.clone();
        self.requests.spawn(
            request_key,
            "zeta-tui-product-command",
            move || Completion::ProductCommand {
                command: invocation.display_text(),
                result: execute_product_command(conversation, &mut client, invocation).and_then(
                    |output| finish_product_command_request(&mut client, subscription, output),
                ),
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
