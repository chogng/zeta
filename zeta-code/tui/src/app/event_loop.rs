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
use crate::components::pane;
use crate::features::additional_directories;
use crate::features::config;
use crate::features::config::ConfigResource;
use crate::features::interactions;
use crate::features::mcp;
use crate::features::rewind;
use crate::features::sessions::ResumeOutcome;
use crate::features::shortcuts::ShortcutResource;
use crate::features::shortcuts::ShortcutResourcePoll;
use crate::features::skills;
use crate::features::status_line::StatusLineResource;
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
use std::time::Instant;
use zeta_app_server_client::AppServerSession;
use zeta_app_server_protocol::protocol::session::ThreadSnapshotHistory;
use zeta_app_server_protocol::protocol::skills::SkillCatalogReloadDto;
use zeta_app_server_protocol::protocol::skills::SkillListParams;

#[path = "config_actions.rs"]
mod config_actions;

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
        keybindings_path,
        status_line_path,
        terminal_settings_path,
        recovery,
    } = options;
    let server_slash_commands = client.initialization()?.slash_commands.clone();
    let slash_registry = client
        .list_skills(SkillListParams {
            reload: SkillCatalogReloadDto::Cached,
            session_id: None,
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
    let (mut thread_subscription, initial_thread, initial_transcript) = ThreadSubscription::start(
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
    app.set_next_approval_mode(conversation.next_approval_mode());
    let now = Instant::now();
    let mut keybindings_resource = keybindings_path.map(|path| ShortcutResource::new(path, now));
    let mut status_line_resource = status_line_path.map(StatusLineResource::new);
    let mut config_resource = terminal_settings_path.map(ConfigResource::new);
    app.replace_slash_commands(slash_registry.catalog, slash_registry.skills);
    apply_thread_snapshot(
        &mut app,
        &mut active_turn,
        initial_thread,
        initial_transcript,
    );
    poll_keybindings_resource(&mut keybindings_resource, &mut app, now);
    if let Some(resource) = config_resource.as_mut() {
        match resource.refresh() {
            Ok(settings) => app.update(AppEvent::ConfigSettingsReceived(settings)),
            Err(error) => app.update(AppEvent::FailureReported(error)),
        }
    }
    if let Some(resource) = status_line_resource.as_mut() {
        match resource.refresh() {
            Ok(settings) => app.update(AppEvent::StatusLineSettingsReceived(settings)),
            Err(error) => app.update(AppEvent::FailureReported(error)),
        }
    }
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
    if let Err(error) = draw_terminal(&mut terminal, &app) {
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
                client::RuntimeEvent::Tick => {
                    let now = Instant::now();
                    app.handle_tick(now);
                    poll_keybindings_resource(&mut keybindings_resource, &mut app, now);
                    None
                }
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
                        activate_pointer_item(&mut app, terminal_area, mouse.column, mouse.row)
                    }
                    Event::Mouse(mouse) if mouse.kind == MouseEventKind::Moved => {
                        let terminal_area = terminal.area()?;
                        select_hovered_popup_item(&mut app, terminal_area, mouse.column, mouse.row);
                        None
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
                            let additional_directory_permissions = config_resource
                                .as_ref()
                                .map(ConfigResource::settings)
                                .unwrap_or_default()
                                .additional_directory_permissions();
                            pending_request = spawn_request(
                                "zeta-tui-product-command",
                                move || {
                                    RequestCompletion::ProductCommand(
                                        execute_product_command(
                                            next_conversation,
                                            &mut request_client,
                                            invocation,
                                            additional_directory_permissions,
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
                    AppCommand::OpenConfigPane => {
                        config_actions::open_config(
                            &mut config_resource,
                            conversation.session_id(),
                            &client,
                            &mut pending_request,
                            &mut app,
                        );
                    }
                    AppCommand::EditConfig(edit) => {
                        config_actions::edit_config(&mut config_resource, &edit, &mut app);
                    }
                    AppCommand::EditAdditionalDirectoryPermissions(edit) => {
                        config_actions::set_additional_directory_permissions(
                            edit,
                            &client,
                            &mut pending_request,
                            &mut app,
                        );
                    }
                    AppCommand::SetProviderApiKey(edit) => {
                        let terminal_snapshot = config_resource
                            .as_ref()
                            .map(|resource| (resource.settings(), resource.revision()))
                            .unwrap_or((config::TerminalSettings::default(), 0));
                        config_actions::set_provider_api_key(
                            edit,
                            terminal_snapshot,
                            conversation.session_id(),
                            &client,
                            &mut pending_request,
                            &mut app,
                        );
                    }
                    AppCommand::OpenShortcutsPane => match keybindings_resource.as_ref() {
                        Some(resource) => {
                            app.update(AppEvent::ShortcutViewOpened(
                                resource.setup_view(&app.app_keymap),
                            ));
                        }
                        None => app.update(AppEvent::FailureReported(
                            "shortcuts are unavailable because no active profile root was configured"
                                .to_owned(),
                        )),
                    },
                    AppCommand::OpenStatusLinePane => match status_line_resource.as_mut() {
                        Some(resource) => match resource.refresh() {
                            Ok(settings) => {
                                app.update(AppEvent::StatusLineSettingsReceived(settings));
                                app.update(AppEvent::StatusLineViewOpened(resource.setup_view()));
                            }
                            Err(error) => app.update(AppEvent::FailureReported(error)),
                        },
                        None => app.update(AppEvent::FailureReported(
                            "status-line settings are unavailable because no active profile root was configured"
                                .to_owned(),
                        )),
                    },
                    AppCommand::EditStatusLine(edit) => match status_line_resource.as_mut() {
                        Some(resource) => match resource.apply_edit(&edit) {
                            Ok((settings, view)) => {
                                app.update(AppEvent::StatusLineSettingsReceived(settings));
                                app.update(AppEvent::StatusLineViewReplaced(view));
                            }
                            Err(error) => app.update(AppEvent::FailureReported(error)),
                        },
                        None => app.update(AppEvent::FailureReported(
                            "status-line settings are unavailable because no active profile root was configured"
                                .to_owned(),
                        )),
                    },
                    AppCommand::EditShortcut(edit) => match keybindings_resource.as_mut() {
                        Some(resource) => match resource.apply_edit(
                            &edit,
                            &mut app.app_keymap,
                            Instant::now(),
                        ) {
                            Ok(notice) => {
                                app.update(AppEvent::ShortcutViewsClosed);
                                app.update(AppEvent::ShortcutViewOpened(
                                    resource.setup_view(&app.app_keymap),
                                ));
                                app.update(AppEvent::HostOperationCompleted(Ok(notice)));
                            }
                            Err(error) => app.update(AppEvent::FailureReported(error)),
                        },
                        None => app.update(AppEvent::FailureReported(
                            "shortcuts are unavailable because no active profile root was configured"
                                .to_owned(),
                        )),
                    },
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
                    AppCommand::RemoveAdditionalDirectory { root } => {
                        if pending_request.is_none() {
                            let mut request_client = client.clone();
                            let session_id = conversation.session_id().clone();
                            let event_root = root.clone();
                            pending_request = spawn_request(
                                "zeta-tui-remove-additional-directory",
                                move || {
                                    RequestCompletion::Presentation(
                                        additional_directories::remove(
                                            &mut request_client,
                                            &session_id,
                                            root,
                                        )
                                        .map(|view| AppEvent::AdditionalDirectoryRemoved {
                                            root: event_root,
                                            view,
                                        })
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
                            let skill_session_id = conversation.session_id().clone();
                            pending_request = spawn_request(
                                "zeta-tui-set-skill-enablement",
                                move || {
                                    RequestCompletion::Presentation(
                                        skills::set_enablement(
                                            &mut request_client,
                                            &skill_session_id,
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
                    AppCommand::CycleNextApprovalMode => {
                        if pending_request.is_none() {
                            let approval_mode = match conversation.next_approval_mode() {
                                zeta_protocol::ApprovalMode::AskPermissions => {
                                    zeta_protocol::ApprovalMode::AutoReview
                                }
                                zeta_protocol::ApprovalMode::AutoReview => {
                                    zeta_protocol::ApprovalMode::BypassPermissions
                                }
                                zeta_protocol::ApprovalMode::BypassPermissions => {
                                    zeta_protocol::ApprovalMode::AskPermissions
                                }
                            };
                            let mut next_conversation = conversation.clone();
                            let mut request_client = client.clone();
                            pending_request = spawn_request(
                                "zeta-tui-set-approval-mode",
                                move || {
                                    let result = next_conversation
                                        .set_next_approval_mode(&mut request_client, approval_mode)
                                        .map(|()| next_conversation);
                                    RequestCompletion::ApprovalModeChanged(result)
                                },
                                &mut app,
                            );
                        }
                    }
                    AppCommand::SubmitTurn { submission } => {
                        draw_terminal(&mut terminal, &app)?;
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
                let skills_session_id = conversation.session_id().clone();
                pending_request = spawn_request(
                    "zeta-tui-refresh-skills",
                    move || {
                        RequestCompletion::SkillsRefreshed(refresh_skills_and_registry(
                            request_client,
                            server_slash_commands,
                            skills_session_id,
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
            draw_terminal(&mut terminal, &app)?;
        }
    })();
    let pump_result = pump.shutdown();
    match (result, pump_result) {
        (Err(error), _) => Err(error),
        (Ok(_), Err(error)) => Err(error.into()),
        (Ok(exit), Ok(())) => Ok(exit),
    }
}

fn activate_pointer_item(
    app: &mut App,
    area: ratatui::layout::Rect,
    column: u16,
    row: u16,
) -> Option<AppCommand> {
    if let Some(index) = selection_tab_index_at(app, area, column, row) {
        app.select_tab(index);
        return None;
    }
    if let Some(index) = selection_item_index_at(app, area, column, row) {
        return app.activate_visible_item(index);
    }
    if let Some(index) = frame::mention_index_at(app, area, column, row) {
        app.activate_mention(index);
        return None;
    }
    frame::slash_command_index_at(app, area, column, row)
        .and_then(|index| app.activate_slash_command(index))
}

fn selection_tab_index_at(
    app: &App,
    area: ratatui::layout::Rect,
    column: u16,
    row: u16,
) -> Option<usize> {
    let view = app.selection_pane()?;
    let frame_areas = ui::frame_areas(
        area,
        ui::InteractionLayout::Expanded {
            desired_height: pane::desired_height(view.body().desired_height(area.width)),
        },
    );
    let pane_areas = pane::areas(frame_areas.interaction);
    view.body().tab_index_at(pane_areas.body, column, row)
}

fn select_hovered_popup_item(app: &mut App, area: ratatui::layout::Rect, column: u16, row: u16) {
    if let Some(index) = selection_item_index_at(app, area, column, row) {
        app.select_visible_item(index);
        return;
    }
    if let Some(index) = frame::mention_index_at(app, area, column, row) {
        app.select_mention(index);
    } else if let Some(index) = frame::slash_command_index_at(app, area, column, row) {
        app.select_slash_command(index);
    }
}

fn selection_item_index_at(
    app: &App,
    area: ratatui::layout::Rect,
    column: u16,
    row: u16,
) -> Option<usize> {
    let view = app.selection_pane()?;
    let frame_areas = ui::frame_areas(
        area,
        ui::InteractionLayout::Expanded {
            desired_height: pane::desired_height(view.body().desired_height(area.width)),
        },
    );
    let pane_areas = pane::areas(frame_areas.interaction);
    view.body().item_index_at(pane_areas.body, column, row)
}

fn draw_terminal(
    terminal: &mut terminal::TerminalSession,
    app: &App,
) -> Result<(), std::io::Error> {
    terminal.set_mouse_mode(app.mouse_mode())?;
    terminal.draw(|terminal_frame| frame::draw(terminal_frame, app))
}

fn poll_keybindings_resource(resource: &mut Option<ShortcutResource>, app: &mut App, now: Instant) {
    let Some(resource) = resource else {
        return;
    };
    match resource.poll(now, &mut app.app_keymap) {
        ShortcutResourcePoll::Unchanged => {}
        ShortcutResourcePoll::Updated => {
            for diagnostic in resource.diagnostics().to_vec() {
                app.report_keybinding_diagnostic(diagnostic);
            }
        }
        ShortcutResourcePoll::Rejected(diagnostic) => {
            app.report_keybinding_diagnostic(diagnostic);
        }
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
            | AppCommand::OpenShortcutsPane
            | AppCommand::OpenStatusLinePane
            | AppCommand::EditShortcut(_)
            | AppCommand::EditStatusLine(_)
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
        client::ClientEvent::SessionUpdated(update) => {
            conversation.apply_session_update(&update);
            app.set_next_approval_mode(conversation.next_approval_mode());
            ServerRefresh::default()
        }
        client::ClientEvent::ThreadUpdated(update) => {
            match thread_subscription.classify_update(&update) {
                ThreadUpdateDisposition::Ignore => ServerRefresh::default(),
                ThreadUpdateDisposition::RefreshSnapshot => ServerRefresh {
                    thread: true,
                    ..ServerRefresh::default()
                },
            }
        }
        client::ClientEvent::ThreadTranscriptUpdated(update) => {
            if update.session_id == *conversation.session_id()
                && update.thread_id == *conversation.thread_id()
            {
                app.update(AppEvent::ThreadTranscriptUpdateReceived(update));
            }
            ServerRefresh::default()
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
