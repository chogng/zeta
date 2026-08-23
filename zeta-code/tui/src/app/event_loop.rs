use super::ActiveConversation;
use super::App;
use super::AppCommand;
use super::AppEvent;
use super::Status;
use super::TuiSlashCommandRegistry;
use super::dispatch::execute_product_command;
use super::frame;
use super::request_completion::RequestCompletion;
use super::request_completion::apply_request_completion;
use super::request_completion::apply_thread_snapshot;
use super::request_completion::finish_conversation_request;
use super::request_completion::finish_product_command_request;
use super::request_completion::interrupt_and_read;
use super::request_completion::refresh_skills_and_registry;
use super::request_completion::resolve_interaction_and_read;
use super::request_completion::start_turn_and_read;
use super::skill_slash_command_registry;
use super::slash_command_registry;
use crate::TuiError;
use crate::TuiExit;
use crate::TuiOptions;
use crate::client;
use crate::features::config;
use crate::features::interactions;
use crate::features::mcp;
use crate::features::rewind;
use crate::features::sessions::ResumeOutcome;
use crate::features::skills;
use crate::features::theme as theme_feature;
use crate::features::thread::ThreadRequestScope;
use crate::features::thread::ThreadSubscription;
use crate::features::thread::ThreadUpdateDisposition;
use crate::features::thread::read_older_thread_history;
use crate::features::thread::read_thread_history;
use crate::features::workspace_files::FileSearchManager;
use crate::host;
use crate::terminal;
use crate::ui;
use crossterm::event::Event;
use crossterm::event::KeyEventKind;
use crossterm::event::MouseButton;
use crossterm::event::MouseEventKind;
use std::collections::VecDeque;
use zeta_app_server_client::AppServerSession;
use zeta_app_server_protocol::protocol::session::ThreadSnapshotHistory;
use zeta_app_server_protocol::protocol::skills::SkillCatalogReloadDto;
use zeta_app_server_protocol::protocol::skills::SkillListParams;

pub(crate) fn run(mut session: AppServerSession, options: TuiOptions) -> Result<TuiExit, TuiError> {
    let result = run_session(&mut session, options);
    let shutdown = session.shutdown();
    match (result, shutdown) {
        (Err(error), _) => Err(error),
        (
            Ok(exit @ (TuiExit::ConnectionLost { .. } | TuiExit::WorkspaceReconnectRequested(_))),
            _,
        ) => Ok(exit),
        (Ok(_), Err(error)) => Err(error.into()),
        (Ok(exit), Ok(())) => Ok(exit),
    }
}

fn run_session(session: &mut AppServerSession, options: TuiOptions) -> Result<TuiExit, TuiError> {
    let mut client = session.client();
    let events = session.take_events()?;
    let TuiOptions {
        thread_title,
        display_workspace_root,
        host_workspace_root,
        host_file_search_root,
        recovery,
    } = options;
    let server_slash_commands = client.initialization()?.slash_commands.clone();
    let slash_registry = client
        .list_skills(SkillListParams {
            reload: SkillCatalogReloadDto::Cached,
        })
        .ok()
        .and_then(|catalog| skill_slash_command_registry(&server_slash_commands, &catalog).ok())
        .unwrap_or(TuiSlashCommandRegistry {
            catalog: slash_command_registry(&server_slash_commands)?,
            skills: Default::default(),
        });
    let mut conversation = match recovery {
        Some(recovery) => ActiveConversation::recover(&mut client, recovery)?,
        None => ActiveConversation::start(&mut client, thread_title)?,
    };
    let mut active_turn = None;
    let (mut thread_subscription, initial_thread) = ThreadSubscription::start(
        &mut client,
        conversation.session_id(),
        conversation.thread_id(),
    )?;
    conversation.set_thread_sequence(initial_thread.sequence);
    let mut terminal = terminal::TerminalSession::open()?;
    crate::ui::configure(terminal.background_color());
    let mut file_search = host_file_search_root.map(FileSearchManager::new);
    let mut app = App::for_workspace_with_slash_commands(
        &display_workspace_root,
        slash_registry.catalog.clone(),
    );
    app.replace_slash_commands(slash_registry.catalog, slash_registry.skills);
    apply_thread_snapshot(&mut app, &mut active_turn, initial_thread);
    if let Ok(config) = client.read_config() {
        app.update(AppEvent::PreferredModelReceived(config.preferred_model));
    }
    if let Ok(status) = client.git_status() {
        app.update(AppEvent::GitStatusReceived(status));
    }

    let pump = client::EventPump::start(events)?;
    let mut pending_request: Option<client::RequestTask<RequestCompletion>> = None;
    let mut queued_actions = VecDeque::new();
    let mut thread_refresh_requested = false;
    let mut skills_refresh_requested = false;
    let mut connectors_refresh_requested = false;
    if let Err(error) = terminal.draw(|terminal_frame| frame::draw(terminal_frame, &app)) {
        let _ = pump.shutdown();
        return Err(error.into());
    }
    let result = (|| {
        loop {
            let action = match pump.recv()? {
                client::RuntimeEvent::Client(event) => {
                    let event = match super::recovery::continue_or_exit(
                        event,
                        conversation.session_id(),
                        conversation.thread_id(),
                    ) {
                        Ok(event) => event,
                        Err(exit) => return Ok(exit),
                    };
                    let refresh = refresh_server_event(
                        event,
                        &mut conversation,
                        &mut active_turn,
                        &mut thread_subscription,
                        &mut app,
                    );
                    thread_refresh_requested |= refresh.thread;
                    skills_refresh_requested |= refresh.skills;
                    connectors_refresh_requested |= refresh.connectors;
                    None
                }
                client::RuntimeEvent::Tick => None,
                client::RuntimeEvent::TerminationRequested => {
                    return Ok(TuiExit::TerminationRequested);
                }
                client::RuntimeEvent::TerminalFailed(error) => return Err(error.into()),
                client::RuntimeEvent::Terminal(event) => match event {
                    Event::Key(key) if key.kind != KeyEventKind::Release => app.handle_key(key),
                    Event::Mouse(mouse)
                        if mouse.kind == MouseEventKind::Down(MouseButton::Left) =>
                    {
                        let terminal_area = terminal.area()?;
                        if let Some(index) =
                            frame::mention_index_at(&app, terminal_area, mouse.column, mouse.row)
                        {
                            app.activate_mention(index);
                            None
                        } else {
                            frame::slash_command_index_at(
                                &app,
                                terminal_area,
                                mouse.column,
                                mouse.row,
                            )
                            .and_then(|index| app.activate_slash_command(index))
                        }
                    }
                    Event::Paste(text) => {
                        app.handle_paste(text);
                        None
                    }
                    _ => None,
                },
            };

            if let Some(task) = pending_request.as_mut() {
                match task.poll() {
                    Ok(Some(completion)) => {
                        pending_request = None;
                        if let Some(exit) = apply_request_completion(
                            completion,
                            &mut conversation,
                            &mut active_turn,
                            &mut thread_subscription,
                            &mut app,
                        ) {
                            return Ok(exit);
                        }
                    }
                    Ok(None) => {}
                    Err(error) => {
                        pending_request = None;
                        app.update(AppEvent::FailureReported(error.to_string()));
                    }
                }
            }

            let action = schedule_action(action, pending_request.is_some(), &mut queued_actions);

            if let Some(file_search) = file_search.as_mut() {
                sync_file_search_query(&app, file_search);
                for snapshot in file_search.poll() {
                    app.update(AppEvent::FileSearchSnapshotReceived(snapshot));
                }
            }

            if let Some(action) = action {
                match action {
                    AppCommand::ConnectConnectorDeviceOAuth {
                        connector_id,
                        connection_generation,
                    } => {
                        if pending_request.is_none() {
                            let mut request_client = client.clone();
                            pending_request = spawn_request(
                                "zeta-tui-connect-device-oauth",
                                move || {
                                    RequestCompletion::Presentation(
                                        crate::features::connectors::connect_device_oauth(
                                            &mut request_client,
                                            connector_id,
                                            connection_generation,
                                        )
                                        .map(AppEvent::ConnectorViewReplaced)
                                        .map_err(|error| error.to_string()),
                                    )
                                },
                                &mut app,
                            );
                        }
                    }
                    AppCommand::ExecuteProductCommand(invocation) => {
                        if pending_request.is_none() {
                            let mut request_client = client.clone();
                            let next_conversation = conversation.clone();
                            let next_subscription = thread_subscription.clone();
                            pending_request = spawn_request(
                                "zeta-tui-product-command",
                                move || {
                                    RequestCompletion::ProductCommand(
                                        execute_product_command(
                                            next_conversation,
                                            &mut request_client,
                                            invocation,
                                        )
                                        .and_then(
                                            |output| {
                                                finish_product_command_request(
                                                    &mut request_client,
                                                    next_subscription,
                                                    output,
                                                )
                                            },
                                        ),
                                    )
                                },
                                &mut app,
                            );
                        }
                    }
                    AppCommand::Quit => return Ok(TuiExit::UserRequested),
                    AppCommand::Interrupt => {
                        if let Some(turn_id) = active_turn.clone()
                            && !matches!(app.status(), Status::Error)
                        {
                            if pending_request.is_none() {
                                let request_client = client.clone();
                                let scope = thread_request_scope(&conversation);
                                let history = thread_subscription.history();
                                pending_request = spawn_request(
                                    "zeta-tui-interrupt-turn",
                                    move || {
                                        RequestCompletion::TurnInterrupted(interrupt_and_read(
                                            request_client,
                                            scope,
                                            turn_id,
                                            history,
                                        ))
                                    },
                                    &mut app,
                                );
                            }
                        } else if !matches!(app.status(), Status::Ready) {
                            app.update(AppEvent::InterruptFailed(
                                "the active turn is not available".into(),
                            ));
                        }
                    }
                    AppCommand::ReadClipboardImage => app.update(AppEvent::ClipboardImageRead(
                        host::clipboard::read_image().map(|image| image.png),
                    )),
                    AppCommand::CopyLastResponse => {
                        let result = app
                            .latest_agent_response()
                            .ok_or_else(|| "there is no Zeta response to copy".to_owned())
                            .and_then(|response| {
                                host::clipboard::write_text(response)
                                    .map(|()| "Copied the latest Zeta response".to_owned())
                            });
                        app.update(AppEvent::HostOperationCompleted(result));
                    }
                    AppCommand::ExportTranscript { requested_path } => {
                        let markdown = app.transcript_markdown();
                        let result = if markdown.is_empty() {
                            Err("there is no conversation to export".to_owned())
                        } else {
                            host::transcript_export::write(
                                &host_workspace_root,
                                requested_path.as_deref(),
                                &markdown,
                            )
                            .map(|path| format!("Exported conversation to {}", path.display()))
                        };
                        app.update(AppEvent::HostOperationCompleted(result));
                    }
                    AppCommand::Suspend => terminal.suspend()?,
                    AppCommand::LoadOlderHistory => {
                        if pending_request.is_none()
                            && let Some(ThreadSnapshotHistory::Before { turn_id, .. }) =
                                thread_subscription.older_history()
                        {
                            let mut request_client = client.clone();
                            let session_id = conversation.session_id().clone();
                            let thread_id = conversation.thread_id().clone();
                            pending_request = spawn_request(
                                "zeta-tui-load-older-history",
                                move || {
                                    RequestCompletion::ThreadHistoryPage(read_older_thread_history(
                                        &mut request_client,
                                        &session_id,
                                        &thread_id,
                                        turn_id,
                                    ))
                                },
                                &mut app,
                            );
                        }
                    }
                    AppCommand::OpenCustomThemePane => match ui::theme_catalog() {
                        Ok(catalog) => app.update(AppEvent::ThemeViewOpened(
                            theme_feature::custom_theme_selection_view(&catalog),
                        )),
                        Err(error) => app.update(AppEvent::FailureReported(error)),
                    },
                    AppCommand::OpenRewindPane => {
                        if pending_request.is_none() {
                            let mut request_client = client.clone();
                            let session_id = conversation.session_id().clone();
                            let thread_id = conversation.thread_id().clone();
                            pending_request = spawn_request(
                                "zeta-tui-load-rewind",
                                move || {
                                    RequestCompletion::Presentation(
                                        rewind::load_selection(
                                            &mut request_client,
                                            &session_id,
                                            &thread_id,
                                        )
                                        .map(AppEvent::RewindViewOpened)
                                        .map_err(|error| error.to_string()),
                                    )
                                },
                                &mut app,
                            );
                        }
                    }
                    AppCommand::OpenWorkspaceDirectory { path } => {
                        if pending_request.is_none() {
                            let mut request_client = client.clone();
                            pending_request = spawn_request(
                                "zeta-tui-read-directory",
                                move || {
                                    RequestCompletion::Presentation(
                                        crate::features::workspace_files::load_directory(
                                            &mut request_client,
                                            path,
                                        )
                                        .map(AppEvent::FileViewOpened)
                                        .map_err(|error| error.to_string()),
                                    )
                                },
                                &mut app,
                            );
                        }
                    }
                    AppCommand::PreviewWorkspaceFile { path } => {
                        if pending_request.is_none() {
                            let mut request_client = client.clone();
                            pending_request = spawn_request(
                                "zeta-tui-read-file-preview",
                                move || {
                                    RequestCompletion::Presentation(
                                        crate::features::workspace_files::load_file_preview(
                                            &mut request_client,
                                            path,
                                        )
                                        .map(AppEvent::SelectionViewOpened)
                                        .map_err(|error| error.to_string()),
                                    )
                                },
                                &mut app,
                            );
                        }
                    }
                    AppCommand::RewindToCheckpoint {
                        before_turn_id,
                        checkpoint_label,
                    } => {
                        let command = format!("/rewind {before_turn_id}");
                        app.update(AppEvent::CommandStarted(command.clone()));
                        if pending_request.is_none() {
                            let mut request_client = client.clone();
                            let mut next_conversation = conversation.clone();
                            let next_subscription = thread_subscription.clone();
                            pending_request = spawn_request(
                                "zeta-tui-rewind-thread",
                                move || RequestCompletion::ConversationChanged {
                                    command,
                                    result: next_conversation
                                        .rewind_active_thread(
                                            &mut request_client,
                                            before_turn_id,
                                            &checkpoint_label,
                                        )
                                        .map_err(|error| error.to_string())
                                        .and_then(|change| {
                                            finish_conversation_request(
                                                &mut request_client,
                                                next_conversation,
                                                next_subscription,
                                                change,
                                            )
                                        }),
                                },
                                &mut app,
                            );
                        }
                    }
                    AppCommand::ResumeSession { session_id } => {
                        let command = format!("/resume {session_id}");
                        app.update(AppEvent::CommandStarted(command.clone()));
                        if pending_request.is_none() {
                            let mut request_client = client.clone();
                            let mut next_conversation = conversation.clone();
                            let next_subscription = thread_subscription.clone();
                            pending_request = spawn_request(
                                "zeta-tui-resume-session",
                                move || match next_conversation
                                    .resume_session(&mut request_client, &session_id)
                                {
                                    Ok(ResumeOutcome::WorkspaceReconnect(reconnect)) => {
                                        RequestCompletion::WorkspaceReconnect(reconnect)
                                    }
                                    Ok(ResumeOutcome::Changed(change)) => {
                                        RequestCompletion::ConversationChanged {
                                            command,
                                            result: finish_conversation_request(
                                                &mut request_client,
                                                next_conversation,
                                                next_subscription,
                                                change,
                                            ),
                                        }
                                    }
                                    Ok(ResumeOutcome::Listed(_)) => {
                                        RequestCompletion::ConversationChanged {
                                            command,
                                            result: Err(
                                                "resume selection did not identify a session"
                                                    .into(),
                                            ),
                                        }
                                    }
                                    Err(error) => RequestCompletion::ConversationChanged {
                                        command,
                                        result: Err(error.to_string()),
                                    },
                                },
                                &mut app,
                            );
                        }
                    }
                    AppCommand::SwitchThread { thread_id } => {
                        let command = format!("/thread {thread_id}");
                        app.update(AppEvent::CommandStarted(command.clone()));
                        if pending_request.is_none() {
                            let mut request_client = client.clone();
                            let mut next_conversation = conversation.clone();
                            let next_subscription = thread_subscription.clone();
                            pending_request = spawn_request(
                                "zeta-tui-switch-thread",
                                move || RequestCompletion::ConversationChanged {
                                    command,
                                    result: next_conversation
                                        .switch_thread(&mut request_client, thread_id)
                                        .map_err(|error| error.to_string())
                                        .and_then(|change| {
                                            finish_conversation_request(
                                                &mut request_client,
                                                next_conversation,
                                                next_subscription,
                                                change,
                                            )
                                        }),
                                },
                                &mut app,
                            );
                        }
                    }
                    AppCommand::ArchiveThread { thread_id } => {
                        let command = format!("/archive-thread {thread_id}");
                        app.update(AppEvent::CommandStarted(command.clone()));
                        if pending_request.is_none() {
                            let mut request_client = client.clone();
                            let mut next_conversation = conversation.clone();
                            let next_subscription = thread_subscription.clone();
                            pending_request = spawn_request(
                                "zeta-tui-archive-thread",
                                move || RequestCompletion::ConversationChanged {
                                    command,
                                    result: next_conversation
                                        .archive_thread(&mut request_client, thread_id)
                                        .map_err(|error| error.to_string())
                                        .and_then(|change| {
                                            finish_conversation_request(
                                                &mut request_client,
                                                next_conversation,
                                                next_subscription,
                                                change,
                                            )
                                        }),
                                },
                                &mut app,
                            );
                        }
                    }
                    AppCommand::ResolveInteraction(response) => {
                        if pending_request.is_none() {
                            let request_client = client.clone();
                            let scope = thread_request_scope(&conversation);
                            let history = thread_subscription.history();
                            pending_request = spawn_request(
                                "zeta-tui-resolve-interaction",
                                move || {
                                    RequestCompletion::InteractionResolved(
                                        resolve_interaction_and_read(
                                            request_client,
                                            scope,
                                            response,
                                            history,
                                        ),
                                    )
                                },
                                &mut app,
                            );
                        }
                    }
                    AppCommand::DisconnectConnector { connector_id } => {
                        if pending_request.is_none() {
                            let mut request_client = client.clone();
                            pending_request = spawn_request(
                                "zeta-tui-disconnect-connector",
                                move || {
                                    RequestCompletion::Presentation(
                                        crate::features::connectors::disconnect(
                                            &mut request_client,
                                            connector_id,
                                        )
                                        .map(AppEvent::ConnectorViewReplaced)
                                        .map_err(|error| error.to_string()),
                                    )
                                },
                                &mut app,
                            );
                        }
                    }
                    AppCommand::SetMcpEnablement {
                        server_id,
                        enablement,
                    } => {
                        if pending_request.is_none() {
                            let mut request_client = client.clone();
                            pending_request = spawn_request(
                                "zeta-tui-set-mcp-enablement",
                                move || {
                                    RequestCompletion::Presentation(
                                        mcp::set_enablement(
                                            &mut request_client,
                                            server_id,
                                            enablement,
                                        )
                                        .map(AppEvent::McpViewReplaced)
                                        .map_err(|error| error.to_string()),
                                    )
                                },
                                &mut app,
                            );
                        }
                    }
                    AppCommand::SetPreferredModel { preference } => {
                        let command = format!("/model {preference}");
                        app.update(AppEvent::CommandStarted(command.clone()));
                        if pending_request.is_none() {
                            let mut request_client = client.clone();
                            pending_request = spawn_request(
                                "zeta-tui-set-preferred-model",
                                move || RequestCompletion::PreferredModelUpdated {
                                    command,
                                    result: config::set_preferred_model(
                                        &mut request_client,
                                        &preference,
                                    )
                                    .map_err(|error| error.to_string()),
                                },
                                &mut app,
                            );
                        }
                    }
                    AppCommand::SetCustomTheme { preference } => {
                        let command = format!("/theme {preference}");
                        app.update(AppEvent::CommandStarted(command.clone()));
                        match ui::select_theme(&preference) {
                            Ok(label) => {
                                app.update(AppEvent::CommandCompleted {
                                    command,
                                    result: format!("Theme set to {label}"),
                                });
                                app.update(AppEvent::ThemeViewClosed);
                            }
                            Err(error) => app.update(AppEvent::FailureReported(error)),
                        }
                    }
                    AppCommand::SetTheme { preference } => {
                        let command = format!("/theme {preference}");
                        app.update(AppEvent::CommandStarted(command.clone()));
                        match ui::select_theme(&preference) {
                            Ok(label) => {
                                app.update(AppEvent::CommandCompleted {
                                    command,
                                    result: format!("Theme set to {label}"),
                                });
                                app.update(AppEvent::ThemeViewClosed);
                            }
                            Err(error) => app.update(AppEvent::FailureReported(error)),
                        }
                    }
                    AppCommand::SetSkillEnablement {
                        skill_id,
                        enablement,
                    } => {
                        if pending_request.is_none() {
                            let mut request_client = client.clone();
                            pending_request = spawn_request(
                                "zeta-tui-set-skill-enablement",
                                move || {
                                    RequestCompletion::Presentation(
                                        skills::set_enablement(
                                            &mut request_client,
                                            skill_id,
                                            enablement,
                                        )
                                        .map(AppEvent::SkillsViewReplaced)
                                        .map_err(|error| error.to_string()),
                                    )
                                },
                                &mut app,
                            );
                        }
                    }
                    AppCommand::SubmitTurn {
                        submission,
                        approval_mode,
                    } => {
                        terminal.draw(|terminal_frame| frame::draw(terminal_frame, &app))?;
                        if pending_request.is_none() {
                            let request_client = client.clone();
                            let scope = thread_request_scope(&conversation);
                            let history = thread_subscription.history();
                            pending_request = spawn_request(
                                "zeta-tui-start-turn",
                                move || {
                                    RequestCompletion::TurnStarted(start_turn_and_read(
                                        request_client,
                                        scope,
                                        submission,
                                        approval_mode,
                                        history,
                                    ))
                                },
                                &mut app,
                            );
                        }
                    }
                }
            }
            if pending_request.is_none() && thread_refresh_requested {
                let mut request_client = client.clone();
                let session_id = conversation.session_id().clone();
                let thread_id = conversation.thread_id().clone();
                let history = thread_subscription.history();
                pending_request = spawn_request(
                    "zeta-tui-refresh-thread",
                    move || {
                        RequestCompletion::ThreadRefreshed(read_thread_history(
                            &mut request_client,
                            &session_id,
                            &thread_id,
                            history,
                        ))
                    },
                    &mut app,
                );
                if pending_request.is_some() {
                    thread_refresh_requested = false;
                }
            }
            if pending_request.is_none() && skills_refresh_requested {
                let request_client = client.clone();
                let server_slash_commands = server_slash_commands.clone();
                pending_request = spawn_request(
                    "zeta-tui-refresh-skills",
                    move || {
                        RequestCompletion::SkillsRefreshed(refresh_skills_and_registry(
                            request_client,
                            server_slash_commands,
                        ))
                    },
                    &mut app,
                );
                if pending_request.is_some() {
                    skills_refresh_requested = false;
                }
            }
            if pending_request.is_none() && connectors_refresh_requested {
                let mut request_client = client.clone();
                pending_request = spawn_request(
                    "zeta-tui-refresh-connectors",
                    move || {
                        RequestCompletion::Presentation(
                            crate::features::connectors::load_selection(&mut request_client)
                                .map(AppEvent::ConnectorViewReplaced)
                                .map_err(|error| error.to_string()),
                        )
                    },
                    &mut app,
                );
                if pending_request.is_some() {
                    connectors_refresh_requested = false;
                }
            }
            terminal.draw(|terminal_frame| frame::draw(terminal_frame, &app))?;
        }
    })();
    let pump_result = pump.shutdown();
    match (result, pump_result) {
        (Err(error), _) => Err(error),
        (Ok(_), Err(error)) => Err(error.into()),
        (Ok(exit), Ok(())) => Ok(exit),
    }
}

fn schedule_action(
    action: Option<AppCommand>,
    request_pending: bool,
    queued: &mut VecDeque<AppCommand>,
) -> Option<AppCommand> {
    if request_pending {
        return match action {
            Some(action) if uses_request_task(&action) => {
                queued.push_back(action);
                None
            }
            action => action,
        };
    }
    if let Some(next) = queued.pop_front() {
        if let Some(action) = action {
            queued.push_back(action);
        }
        Some(next)
    } else {
        action
    }
}

fn uses_request_task(action: &AppCommand) -> bool {
    !matches!(
        action,
        AppCommand::Quit
            | AppCommand::Suspend
            | AppCommand::CopyLastResponse
            | AppCommand::ExportTranscript { .. }
            | AppCommand::ReadClipboardImage
            | AppCommand::OpenCustomThemePane
            | AppCommand::SetCustomTheme { .. }
            | AppCommand::SetTheme { .. }
    )
}

#[derive(Default)]
struct ServerRefresh {
    connectors: bool,
    thread: bool,
    skills: bool,
}

fn refresh_server_event(
    event: client::ClientEvent,
    conversation: &mut ActiveConversation,
    active_turn: &mut Option<zeta_protocol::TurnId>,
    thread_subscription: &mut ThreadSubscription,
    app: &mut App,
) -> ServerRefresh {
    match event {
        client::ClientEvent::AgentRequest(request) => {
            if request.session_id == *conversation.session_id()
                && request.thread_id == *conversation.thread_id()
            {
                *active_turn = Some(request.turn_id.clone());
                match interactions::interaction_selection_view(*request) {
                    Ok(view) => app.update(AppEvent::InteractionViewOpened(view)),
                    Err(error) => app.update(AppEvent::FailureReported(error)),
                }
            }
            ServerRefresh::default()
        }
        client::ClientEvent::SkillsChanged => ServerRefresh {
            skills: true,
            ..ServerRefresh::default()
        },
        client::ClientEvent::ConnectorsChanged => ServerRefresh {
            connectors: app.connector_view_open(),
            ..ServerRefresh::default()
        },
        client::ClientEvent::PackageSourcesChanged => ServerRefresh {
            connectors: app.connector_view_open(),
            skills: true,
            ..ServerRefresh::default()
        },
        client::ClientEvent::ConnectionClosed(_) => {
            unreachable!("connection failures leave through the recovery boundary")
        }
        client::ClientEvent::GitStatusChanged(status) => {
            app.update(AppEvent::GitStatusReceived(status));
            ServerRefresh::default()
        }
        client::ClientEvent::ThreadUpdated(update) => {
            match thread_subscription.classify_update(&update) {
                ThreadUpdateDisposition::ApplyTransient => {
                    app.update(AppEvent::TransientThreadUpdateReceived(update));
                    ServerRefresh::default()
                }
                ThreadUpdateDisposition::ApplyTransientAfterReset => {
                    app.update(AppEvent::TransientThreadStreamReset);
                    app.update(AppEvent::TransientThreadUpdateReceived(update));
                    ServerRefresh::default()
                }
                ThreadUpdateDisposition::Ignore => ServerRefresh::default(),
                ThreadUpdateDisposition::RefreshSnapshot => ServerRefresh {
                    thread: true,
                    ..ServerRefresh::default()
                },
                ThreadUpdateDisposition::ResetTransientAndRefreshSnapshot => {
                    app.update(AppEvent::TransientThreadStreamReset);
                    ServerRefresh {
                        thread: true,
                        ..ServerRefresh::default()
                    }
                }
            }
        }
    }
}

fn spawn_request(
    name: &'static str,
    request: impl FnOnce() -> RequestCompletion + Send + 'static,
    app: &mut App,
) -> Option<client::RequestTask<RequestCompletion>> {
    match client::RequestTask::spawn(name, request) {
        Ok(task) => Some(task),
        Err(error) => {
            app.update(AppEvent::FailureReported(format!(
                "could not start background request: {error}"
            )));
            None
        }
    }
}

fn thread_request_scope(conversation: &ActiveConversation) -> ThreadRequestScope {
    ThreadRequestScope::new(
        conversation.session_id(),
        conversation.thread_id(),
        conversation.thread_sequence(),
    )
}

fn sync_file_search_query(app: &App, file_search: &mut FileSearchManager) {
    if let Some(query) = app.mention_query() {
        file_search.update_query(query);
    } else {
        file_search.stop();
    }
}

#[cfg(test)]
#[path = "event_loop_tests.rs"]
mod tests;
