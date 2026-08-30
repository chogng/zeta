use super::ActiveConversation;
use super::App;
use super::AppCommand;
use super::AppEvent;
use super::Status;
use super::chat_input_catalog_snapshot;
use super::dispatch::execute_product_command;
use super::event_pump::EventPump;
use super::event_pump::RuntimeEvent;
use super::frame;
use super::frame::InputPointerTarget;
use super::redraw::RedrawPriority;
use super::redraw::RedrawScheduler;
use super::request_completion::RequestCompletion;
use super::request_completion::apply_request_completion;
use super::request_completion::apply_thread_snapshot;
use super::request_completion::create_manager_session_and_start;
use super::request_completion::finish_conversation_request;
use super::request_completion::finish_product_command_request;
use super::request_completion::interrupt_and_read;
use super::request_completion::refresh_skills_and_registry;
use super::request_completion::resolve_thread_request_and_read;
use super::request_completion::start_turn_and_read;
use super::request_completion::steer_turn_and_read;
use super::slash_command_registry;
use super::transcript_batch::TranscriptBatch;
use crate::TuiError;
use crate::TuiExit;
use crate::TuiOptions;
use crate::client;
use crate::components::chat_composer::ChatComposerPointerTarget;
use crate::components::chat_input::ChatInputCatalog;
use crate::features::approval::Approval;
use crate::features::config;
use crate::features::dirs;
use crate::features::file_search::FileSearchManager;
use crate::features::keymap::KeymapResource;
use crate::features::keymap::KeymapResourcePoll;
use crate::features::mcp;
use crate::features::query::Query;
use crate::features::rewind;
use crate::features::sessions;
use crate::features::sessions::ResumeOutcome;
use crate::features::skills;
use crate::features::status_line::StatusLineResource;
use crate::features::theme as theme_feature;
use crate::features::theme::ThemeResource;
use crate::features::thread::ThreadRequestScope;
use crate::features::thread::ThreadSubscription;
use crate::features::thread::ThreadUpdateDisposition;
use crate::features::thread::TranscriptUpdateDisposition;
use crate::features::thread::read_older_thread_history;
use crate::features::thread::read_thread_history;
use crate::host;
use crate::screen_selection::ClickCount;
use crate::screen_selection::ScreenSelectionOutcome;
use crate::terminal;
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

pub(crate) fn run(mut session: AppServerSession, options: TuiOptions) -> Result<TuiExit, TuiError> {
    let result = run_session(&mut session, options);
    let shutdown = session.shutdown();
    match (result, shutdown) {
        (Err(error), _) => Err(error),
        (Ok(exit @ TuiExit::ConnectionLost { .. }), _) => Ok(exit),
        (Ok(_), Err(error)) => Err(error.into()),
        (Ok(exit), Ok(())) => Ok(exit),
    }
}

fn run_session(session: &mut AppServerSession, options: TuiOptions) -> Result<TuiExit, TuiError> {
    let mut client = session.client();
    let events = session.take_events()?;
    let TuiOptions {
        thread_title,
        display_dir_root,
        host_dir_root,
        host_file_search_root,
        keybindings_path,
        status_line_path,
        theme_root,
        recovery,
    } = options;
    let initialization = client.initialization()?;
    let server_slash_commands = initialization.slash_commands.clone();
    let plugins_enabled = initialization.capabilities.plugins;
    let plugins = if plugins_enabled {
        client.list_plugins()?.packages
    } else {
        Vec::new()
    };
    let initial_skill_catalog = client
        .list_skills(SkillListParams {
            reload: SkillCatalogReloadDto::Cached,
            session_id: None,
        })
        .ok();
    let input_catalog = initial_skill_catalog
        .as_ref()
        .and_then(|catalog| {
            chat_input_catalog_snapshot(&server_slash_commands, catalog, &plugins).ok()
        })
        .unwrap_or(ChatInputCatalog::with_slash_commands(
            slash_command_registry(&server_slash_commands)?,
        ));
    let initial_skill_diagnostics = initial_skill_catalog
        .map(|catalog| catalog.diagnostics)
        .unwrap_or_default();
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
    let theme_resource = match theme_root {
        Some(theme_root) => ThemeResource::in_product_root(theme_root, terminal.background_color()),
        None => ThemeResource::new(terminal.background_color()),
    };
    let mut file_search = host_file_search_root.map(FileSearchManager::new);
    let mut app = App::for_dir_with_input_catalog(&display_dir_root, input_catalog);
    let initial_config = client.read_config();
    let theme_preference = initial_config
        .as_ref()
        .map(config::tui_theme)
        .unwrap_or("system");
    match theme_resource.load(theme_preference) {
        Ok(loaded) => {
            for diagnostic in loaded.diagnostics {
                eprintln!("theme: {diagnostic}");
            }
            app.update(AppEvent::RenderThemeChanged(loaded.theme));
        }
        Err(error) => app.update(AppEvent::FailureReported(error)),
    }
    match initial_config {
        Ok(config) => {
            match config::TerminalSettings::from_tui(&config.tui) {
                Ok(settings) => app.update(AppEvent::ConfigSettingsReceived(settings)),
                Err(error) => app.update(AppEvent::FailureReported(error)),
            }
            app.update(AppEvent::PreferredModelReceived(config.preferred_model));
        }
        Err(error) => app.update(AppEvent::FailureReported(format!(
            "could not read server configuration: {error}"
        ))),
    }
    match sessions::load_catalog(&mut client) {
        Ok(catalog) => app.update(AppEvent::SessionCatalogReceived(catalog)),
        Err(error) => app.update(AppEvent::FailureReported(format!(
            "could not load Sessions: {error}"
        ))),
    }
    let now = Instant::now();
    let mut keymap_resource = keybindings_path.map(|path| KeymapResource::new(path, now));
    let mut status_line_resource = status_line_path.map(StatusLineResource::new);
    apply_thread_snapshot(
        &mut app,
        &mut active_turn,
        initial_thread,
        initial_transcript,
    );
    app.update(AppEvent::SkillDiagnosticsReceived(
        initial_skill_diagnostics,
    ));
    poll_keymap_resource(&mut keymap_resource, &mut app, now);
    if let Some(resource) = status_line_resource.as_mut() {
        match resource.refresh() {
            Ok(settings) => app.update(AppEvent::StatusLineSettingsReceived(settings)),
            Err(error) => app.update(AppEvent::FailureReported(error)),
        }
    }
    if let Ok(status) = client.git_status() {
        app.update(AppEvent::GitStatusReceived(status));
    }

    let pump = EventPump::start(events)?;
    let mut pending_request: Option<client::RequestTask<RequestCompletion>> = None;
    let mut queued_actions = VecDeque::new();
    let mut thread_refresh_requested = false;
    let mut skills_refresh_requested = false;
    let mut connectors_refresh_requested = false;
    let mut sessions_refresh_requested = false;
    let mut queued_turn_dispatch_requested = false;
    let mut redraw = RedrawScheduler::default();
    let mut pending_runtime_event = None;
    if let Err(error) = draw_terminal(&mut terminal, &app) {
        let _ = pump.shutdown();
        return Err(error.into());
    }
    let result = (|| {
        loop {
            let had_active_turn = active_turn.is_some();
            let mut runtime_event = match pending_runtime_event.take() {
                Some(event) => event,
                None => match redraw.wait_timeout(Instant::now()) {
                    Some(timeout) => match pump.recv_timeout(timeout)? {
                        Some(event) => event,
                        None => {
                            if redraw.take_due(Instant::now()) {
                                draw_terminal(&mut terminal, &app)?;
                            }
                            continue;
                        }
                    },
                    None => pump.recv()?,
                },
            };
            match &runtime_event {
                RuntimeEvent::Client(_) => {
                    redraw.request(Instant::now(), RedrawPriority::Batched);
                }
                RuntimeEvent::Terminal(terminal::TerminalEvent::Input(_)) => {
                    redraw.request(Instant::now(), RedrawPriority::Immediate);
                }
                RuntimeEvent::Terminal(terminal::TerminalEvent::Tick)
                | RuntimeEvent::Terminal(terminal::TerminalEvent::Failed(_))
                | RuntimeEvent::TerminationRequested => {}
            }
            runtime_event = match runtime_event {
                RuntimeEvent::Client(client::ClientEvent::ThreadTranscriptUpdated(update)) => {
                    match TranscriptBatch::start(*update) {
                        Ok(mut batch) => {
                            while let Some(timeout) = redraw.wait_timeout(Instant::now()) {
                                if timeout.is_zero() {
                                    break;
                                }
                                let Some(next) = pump.recv_timeout(timeout)? else {
                                    break;
                                };
                                match next {
                                    RuntimeEvent::Client(
                                        client::ClientEvent::ThreadTranscriptUpdated(update),
                                    ) => match batch.push(*update) {
                                        Ok(()) => {}
                                        Err(update) => {
                                            pending_runtime_event = Some(RuntimeEvent::Client(
                                                client::ClientEvent::ThreadTranscriptUpdated(
                                                    Box::new(update),
                                                ),
                                            ));
                                            break;
                                        }
                                    },
                                    event => {
                                        pending_runtime_event = Some(event);
                                        break;
                                    }
                                }
                            }
                            RuntimeEvent::Client(client::ClientEvent::ThreadTranscriptUpdated(
                                Box::new(batch.finish()),
                            ))
                        }
                        Err(update) => RuntimeEvent::Client(
                            client::ClientEvent::ThreadTranscriptUpdated(Box::new(update)),
                        ),
                    }
                }
                event => event,
            };
            let action = match runtime_event {
                RuntimeEvent::Client(event) => {
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
                    sessions_refresh_requested |= refresh.sessions;
                    None
                }
                RuntimeEvent::TerminationRequested => {
                    return Ok(TuiExit::TerminationRequested);
                }
                RuntimeEvent::Terminal(terminal::TerminalEvent::Failed(error)) => {
                    return Err(error.into());
                }
                RuntimeEvent::Terminal(terminal::TerminalEvent::Tick) => {
                    let now = Instant::now();
                    let app_changed = app.handle_tick(now);
                    let keymap_changed = poll_keymap_resource(&mut keymap_resource, &mut app, now);
                    if app_changed || keymap_changed {
                        redraw.request(now, RedrawPriority::Batched);
                    }
                    None
                }
                RuntimeEvent::Terminal(terminal::TerminalEvent::Input(event)) => match event {
                    Event::Key(key) if key.kind != KeyEventKind::Release => app.handle_key(key),
                    Event::Mouse(mouse)
                        if mouse.kind == MouseEventKind::Down(MouseButton::Left) =>
                    {
                        let terminal_area = terminal.area()?;
                        app.update_pointer_pressed(frame::input_pointer_target_at(
                            &app,
                            terminal_area,
                            mouse.column,
                            mouse.row,
                        ));
                        app.begin_screen_selection(ratatui::layout::Position::new(
                            mouse.column,
                            mouse.row,
                        ));
                        None
                    }
                    Event::Mouse(mouse)
                        if mouse.kind == MouseEventKind::Drag(MouseButton::Left) =>
                    {
                        app.clear_pointer_pressed();
                        app.drag_screen_selection(ratatui::layout::Position::new(
                            mouse.column,
                            mouse.row,
                        ));
                        None
                    }
                    Event::Mouse(mouse) if mouse.kind == MouseEventKind::Up(MouseButton::Left) => {
                        finish_pointer_gesture(
                            &mut app,
                            &terminal,
                            ratatui::layout::Position::new(mouse.column, mouse.row),
                        )?
                    }
                    Event::Mouse(mouse) if mouse.kind == MouseEventKind::Moved => {
                        let terminal_area = terminal.area()?;
                        update_pointer_hover(&mut app, terminal_area, mouse.column, mouse.row);
                        None
                    }
                    Event::Mouse(mouse)
                        if matches!(
                            mouse.kind,
                            MouseEventKind::ScrollUp | MouseEventKind::ScrollDown
                        ) =>
                    {
                        app.scroll_session_manager(mouse.kind == MouseEventKind::ScrollUp);
                        None
                    }
                    Event::Paste(text) => {
                        app.handle_paste(text);
                        None
                    }
                    Event::Resize(_, _) => {
                        app.clear_pointer_interaction();
                        None
                    }
                    _ => None,
                },
            };

            if let Some(task) = pending_request.as_mut() {
                match task.poll() {
                    Ok(Some(completion)) => {
                        pending_request = None;
                        redraw.request(Instant::now(), RedrawPriority::Batched);
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
                        redraw.request(Instant::now(), RedrawPriority::Batched);
                        app.update(AppEvent::FailureReported(error.to_string()));
                    }
                }
            }

            queued_turn_dispatch_requested |= had_active_turn && active_turn.is_none();

            let mut action =
                schedule_action(action, pending_request.is_some(), &mut queued_actions);
            if action.is_none()
                && pending_request.is_none()
                && queued_actions.is_empty()
                && queued_turn_dispatch_requested
            {
                action = app.dispatch_next_queued_turn();
                queued_turn_dispatch_requested = false;
            }

            if let Some(file_search) = file_search.as_mut() {
                sync_file_search_query(&app, file_search);
                let snapshots = file_search.poll();
                if !snapshots.is_empty() {
                    redraw.request(Instant::now(), RedrawPriority::Batched);
                }
                for snapshot in snapshots {
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
                                        .map(AppEvent::ConnectorPaneReplaced)
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
                            let dir_permissions = app.terminal_settings().dir_permissions();
                            pending_request = spawn_request(
                                "zeta-tui-product-command",
                                move || {
                                    RequestCompletion::ProductCommand(
                                        execute_product_command(
                                            next_conversation,
                                            &mut request_client,
                                            invocation,
                                            dir_permissions,
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
                                let char_count = response.chars().count();
                                host::clipboard::write_text(response)
                                    .map(|()| char_count)
                            });
                        match result {
                            Ok(char_count) => app.update(AppEvent::StatusNoticeShown(format!(
                                "Copied {char_count} chars to clipboard"
                            ))),
                            Err(error) => {
                                app.update(AppEvent::HostOperationCompleted(Err(error)))
                            }
                        }
                    }
                    AppCommand::OpenConfigPane => {
                        if pending_request.is_none() {
                            let mut request_client = client.clone();
                            let session_id = conversation.session_id().clone();
                            pending_request = spawn_request(
                                "zeta-tui-read-config",
                                move || {
                                    RequestCompletion::Presentation(
                                        config::read_config_pane(
                                            &mut request_client,
                                            &session_id,
                                        )
                                        .map(AppEvent::ConfigPaneOpened)
                                        .map_err(|error| error.to_string()),
                                    )
                                },
                                &mut app,
                            );
                        }
                    }
                    AppCommand::EditConfig(edit) => {
                        if pending_request.is_none() {
                            let mut request_client = client.clone();
                            pending_request = spawn_request(
                                "zeta-tui-set-config",
                                move || {
                                    RequestCompletion::Presentation(
                                        config::set_terminal_settings(&mut request_client, edit)
                                            .map(AppEvent::ConfigUpdated)
                                            .map_err(|error| error.to_string()),
                                    )
                                },
                                &mut app,
                            );
                        }
                    }
                    AppCommand::EditPermissions(edit) => {
                        if pending_request.is_none() {
                            let mut request_client = client.clone();
                            pending_request = spawn_request(
                                "zeta-tui-set-directory-permissions",
                                move || {
                                    RequestCompletion::Presentation(
                                        config::set_permissions(
                                            &mut request_client,
                                            edit,
                                        )
                                        .map(AppEvent::ConfigPaneReplaced)
                                        .map_err(|error| error.to_string()),
                                    )
                                },
                                &mut app,
                            );
                        }
                    }
                    AppCommand::SetProviderApiKey(edit) => {
                        if pending_request.is_none() {
                            let mut request_client = client.clone();
                            let session_id = conversation.session_id().clone();
                            pending_request = spawn_request(
                                "zeta-tui-set-provider-api-key",
                                move || {
                                    RequestCompletion::Presentation(
                                        config::set_provider_api_key(
                                            &mut request_client,
                                            edit,
                                            &session_id,
                                        )
                                        .map(|update| AppEvent::ConfigApiKeySaved {
                                            provider: update.provider,
                                            pane_spec: update.pane_spec,
                                        })
                                        .map_err(|error| error.to_string()),
                                    )
                                },
                                &mut app,
                            );
                        }
                    }
                    AppCommand::OpenKeymapPane => match keymap_resource.as_ref() {
                        Some(resource) => {
                            app.update(AppEvent::KeymapPaneOpened(
                                resource.pane_spec(&app.app_keymap),
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
                                app.update(AppEvent::StatusLinePaneOpened(resource.setup_pane_spec()));
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
                                app.update(AppEvent::StatusLinePaneReplaced(view));
                            }
                            Err(error) => app.update(AppEvent::FailureReported(error)),
                        },
                        None => app.update(AppEvent::FailureReported(
                            "status-line settings are unavailable because no active profile root was configured"
                                .to_owned(),
                        )),
                    },
                    AppCommand::EditKeymap(edit) => match keymap_resource.as_mut() {
                        Some(resource) => match resource.apply_edit(
                            &edit,
                            &mut app.app_keymap,
                            Instant::now(),
                        ) {
                            Ok(notice) => {
                                app.update(AppEvent::KeymapPanesClosed);
                                app.update(AppEvent::KeymapPaneOpened(
                                    resource.pane_spec(&app.app_keymap),
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
                                &host_dir_root,
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
                    AppCommand::OpenThemePane => {
                        if pending_request.is_none() {
                            let mut request_client = client.clone();
                            let request_theme_resource = theme_resource.clone();
                            pending_request = spawn_request(
                                "zeta-tui-open-theme",
                                move || {
                                    RequestCompletion::Presentation(
                                        request_client
                                            .read_config()
                                            .map_err(|error| error.to_string())
                                            .and_then(|config| {
                                                request_theme_resource
                                                    .catalog(config::tui_theme(&config))
                                                    .map(|catalog| {
                                                        AppEvent::ThemePaneOpened(
                                                            theme_feature::theme_pane_spec(&catalog),
                                                        )
                                                    })
                                            }),
                                    )
                                },
                                &mut app,
                            );
                        }
                    }
                    AppCommand::OpenCustomThemePane => {
                        if pending_request.is_none() {
                            let mut request_client = client.clone();
                            let request_theme_resource = theme_resource.clone();
                            pending_request = spawn_request(
                                "zeta-tui-open-custom-theme",
                                move || {
                                    RequestCompletion::Presentation(
                                        request_client
                                            .read_config()
                                            .map_err(|error| error.to_string())
                                            .and_then(|config| {
                                                request_theme_resource
                                                    .catalog(config::tui_theme(&config))
                                                    .map(|catalog| {
                                                        AppEvent::ThemePaneOpened(
                                                            theme_feature::custom_theme_pane_spec(
                                                                &catalog,
                                                            ),
                                                        )
                                                    })
                                            }),
                                    )
                                },
                                &mut app,
                            );
                        }
                    }
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
                                        .map(AppEvent::RewindPaneOpened)
                                        .map_err(|error| error.to_string()),
                                    )
                                },
                                &mut app,
                            );
                        }
                    }
                    AppCommand::RemoveDir { path } => {
                        if pending_request.is_none() {
                            let mut request_client = client.clone();
                            let session_id = conversation.session_id().clone();
                            let event_path = path.clone();
                            pending_request = spawn_request(
                                "zeta-tui-remove-directory",
                                move || {
                                    RequestCompletion::Presentation(
                                        dirs::remove(
                                            &mut request_client,
                                            &session_id,
                                            path,
                                        )
                                        .map(|pane_spec| AppEvent::DirRemoved {
                                            path: event_path,
                                            pane_spec,
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
                    AppCommand::ResumeSession {
                        session_id,
                        preferred_thread_id,
                    } => {
                        let command = format!("/resume {session_id}");
                        app.update(AppEvent::CommandStarted(command.clone()));
                        if pending_request.is_none() {
                            let mut request_client = client.clone();
                            let mut next_conversation = conversation.clone();
                            let next_subscription = thread_subscription.clone();
                            pending_request = spawn_request(
                                "zeta-tui-resume-session",
                                move || match next_conversation
                                    .resume_session(
                                        &mut request_client,
                                        &session_id,
                                        preferred_thread_id.as_ref(),
                                    )
                                {
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
                    AppCommand::ArchiveSessions { session_ids } => {
                        if pending_request.is_none() {
                            let mut request_client = client.clone();
                            pending_request = spawn_request(
                                "zeta-tui-archive-sessions",
                                move || {
                                    RequestCompletion::Presentation(
                                        sessions::archive(&mut request_client, session_ids)
                                            .map(AppEvent::SessionCatalogReceived)
                                            .map_err(|error| error.to_string()),
                                    )
                                },
                                &mut app,
                            );
                        }
                    }
                    AppCommand::ResolveThreadRequest(response) => {
                        if pending_request.is_none() {
                            let request = response.identity();
                            let request_client = client.clone();
                            let scope = thread_request_scope(&conversation);
                            let history = thread_subscription.history();
                            pending_request = spawn_request(
                                "zeta-tui-resolve-thread-request",
                                move || {
                                    RequestCompletion::ThreadRequestResolved {
                                        request,
                                        result: resolve_thread_request_and_read(
                                            request_client,
                                            scope,
                                            response,
                                            history,
                                        ),
                                    }
                                },
                                &mut app,
                            );
                        }
                    }
                    AppCommand::CreateSessionAndEnter { submission } => {
                        if pending_request.is_none() {
                            let request_client = client.clone();
                            let next_conversation = conversation.clone();
                            let next_subscription = thread_subscription.clone();
                            let approval_mode = app.approval_mode();
                            pending_request = spawn_request(
                                "zeta-tui-create-manager-session",
                                move || {
                                    RequestCompletion::ManagerSessionCreated(
                                        create_manager_session_and_start(
                                            request_client,
                                            next_conversation,
                                            next_subscription,
                                            submission,
                                            approval_mode,
                                        ),
                                    )
                                },
                                &mut app,
                            );
                        }
                    }
                    AppCommand::SwitchThread { thread_id } => {
                        if pending_request.is_none() {
                            let mut request_client = client.clone();
                            let mut next_conversation = conversation.clone();
                            let next_subscription = thread_subscription.clone();
                            pending_request = spawn_request(
                                "zeta-tui-switch-thread",
                                move || {
                                    let result = next_conversation
                                        .select_thread(&mut request_client, thread_id)
                                        .map_err(|error| error.to_string())
                                        .and_then(|change| {
                                            finish_conversation_request(
                                                &mut request_client,
                                                next_conversation,
                                                next_subscription,
                                                change,
                                            )
                                        });
                                    RequestCompletion::ThreadChanged(result)
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
                                        .map(AppEvent::ConnectorPaneReplaced)
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
                                        .map(AppEvent::McpPaneReplaced)
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
                    AppCommand::SetCustomTheme { preference }
                    | AppCommand::SetTheme { preference } => {
                        let command = format!("/theme {preference}");
                        app.update(AppEvent::CommandStarted(command.clone()));
                        match theme_resource.resolve(&preference) {
                            Ok(selection) => {
                                if pending_request.is_none() {
                                    let mut request_client = client.clone();
                                    pending_request = spawn_request(
                                        "zeta-tui-set-theme",
                                        move || RequestCompletion::ThemeUpdated {
                                            command,
                                            label: selection.label,
                                            theme: selection.theme,
                                            result: config::set_tui_theme(
                                                &mut request_client,
                                                preference,
                                            )
                                            .map_err(|error| error.to_string()),
                                        },
                                        &mut app,
                                    );
                                }
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
                                        .map(AppEvent::SkillsPaneReplaced)
                                        .map_err(|error| error.to_string()),
                                    )
                                },
                                &mut app,
                            );
                        }
                    }
                    AppCommand::CycleNextApprovalMode => {
                        app.cycle_next_approval_mode();
                    }
                    AppCommand::SubmitTurn { submission } => {
                        if pending_request.is_none() {
                            let request_client = client.clone();
                            let scope = thread_request_scope(&conversation);
                            let approval_mode = app.approval_mode();
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
                    AppCommand::SubmitQueuedTurn {
                        queue_id,
                        submission,
                    } => {
                        if pending_request.is_none() {
                            let request_client = client.clone();
                            let scope = thread_request_scope(&conversation);
                            let approval_mode = app.approval_mode();
                            let history = thread_subscription.history();
                            pending_request = spawn_request(
                                "zeta-tui-start-queued-turn",
                                move || RequestCompletion::QueuedTurnStarted {
                                    queue_id,
                                    result: start_turn_and_read(
                                        request_client,
                                        scope,
                                        submission,
                                        approval_mode,
                                        history,
                                    ),
                                },
                                &mut app,
                            );
                        }
                    }
                    AppCommand::SteerTurn {
                        steer_id,
                        submission,
                    } => {
                        if pending_request.is_none() {
                            if matches!(app.status(), Status::Working)
                                && !app.steers_active_turn()
                            {
                                queued_actions.push_front(AppCommand::SteerTurn {
                                    steer_id,
                                    submission,
                                });
                            } else if let Some(turn_id) = active_turn.clone() {
                                let request_client = client.clone();
                                let scope = thread_request_scope(&conversation);
                                let history = thread_subscription.history();
                                pending_request = spawn_request(
                                    "zeta-tui-steer-turn",
                                    move || RequestCompletion::TurnSteered {
                                        steer_id,
                                        result: steer_turn_and_read(
                                            request_client,
                                            scope,
                                            turn_id,
                                            submission,
                                            history,
                                        ),
                                    },
                                    &mut app,
                                );
                            } else {
                                app.update(AppEvent::SteerSubmissionFailed {
                                    steer_id,
                                    error: "the active Turn is no longer available".into(),
                                });
                            }
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
                            plugins_enabled,
                        ))
                    },
                    &mut app,
                );
                if pending_request.is_some() {
                    skills_refresh_requested = false;
                }
            }
            if pending_request.is_none() && sessions_refresh_requested {
                let mut request_client = client.clone();
                pending_request = spawn_request(
                    "zeta-tui-refresh-sessions",
                    move || {
                        RequestCompletion::Presentation(
                            sessions::load_catalog(&mut request_client)
                                .map(AppEvent::SessionCatalogReceived)
                                .map_err(|error| error.to_string()),
                        )
                    },
                    &mut app,
                );
                if pending_request.is_some() {
                    sessions_refresh_requested = false;
                }
            }
            if pending_request.is_none() && connectors_refresh_requested {
                let mut request_client = client.clone();
                pending_request = spawn_request(
                    "zeta-tui-refresh-connectors",
                    move || {
                        RequestCompletion::Presentation(
                            crate::features::connectors::load_selection(&mut request_client)
                                .map(AppEvent::ConnectorPaneReplaced)
                                .map_err(|error| error.to_string()),
                        )
                    },
                    &mut app,
                );
                if pending_request.is_some() {
                    connectors_refresh_requested = false;
                }
            }
            if redraw.take_due(Instant::now()) {
                draw_terminal(&mut terminal, &app)?;
            }
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
    let target = frame::input_pointer_target_at(app, area, column, row)?;
    match target {
        InputPointerTarget::Composer(ChatComposerPointerTarget::PaneTab(index)) => {
            app.select_tab(index);
            None
        }
        InputPointerTarget::Composer(ChatComposerPointerTarget::PaneSearch) => {
            app.focus_pane_search();
            None
        }
        InputPointerTarget::Composer(ChatComposerPointerTarget::PaneItem(index)) => {
            app.activate_visible_item(index)
        }
        InputPointerTarget::Composer(ChatComposerPointerTarget::CompletionItem(index)) => {
            app.activate_input_completion(index)
        }
        InputPointerTarget::Approval(index) | InputPointerTarget::Query(index) => {
            app.activate_thread_request_choice(index)
        }
        InputPointerTarget::SessionManager(target) => {
            app.activate_session_manager_pointer_target(target);
            None
        }
        InputPointerTarget::TranscriptToggle(entry_id) => {
            app.toggle_transcript_cell(&entry_id);
            None
        }
        InputPointerTarget::TranscriptDetails(entry_id) => {
            app.open_transcript_cell_details(&entry_id);
            None
        }
    }
}

fn finish_pointer_gesture(
    app: &mut App,
    terminal: &terminal::TerminalSession,
    position: ratatui::layout::Position,
) -> Result<Option<AppCommand>, std::io::Error> {
    let outcome = app.finish_screen_selection(position, Instant::now());
    app.clear_pointer_pressed();
    match outcome {
        Some(ScreenSelectionOutcome::Click {
            position,
            count: ClickCount::Single,
        }) => {
            let area = terminal.area()?;
            Ok(activate_pointer_item(app, area, position.x, position.y))
        }
        Some(ScreenSelectionOutcome::Click {
            position,
            count: ClickCount::Double,
        }) => {
            if let Some(range) = terminal.token_range_at(position) {
                copy_screen_range(app, terminal, range);
            }
            Ok(None)
        }
        Some(ScreenSelectionOutcome::Click {
            position,
            count: ClickCount::Triple,
        }) => {
            if let Some(range) = terminal.line_range_at(position) {
                copy_screen_range(app, terminal, range);
            }
            Ok(None)
        }
        Some(ScreenSelectionOutcome::Copy(range)) => {
            copy_screen_range(app, terminal, range);
            Ok(None)
        }
        None => Ok(None),
    }
}

fn copy_screen_range(
    app: &mut App,
    terminal: &terminal::TerminalSession,
    range: crate::screen_selection::ScreenSelectionRange,
) {
    app.select_screen_range(range);
    let Some(text) = terminal.selected_text(range) else {
        return;
    };
    let char_count = text.chars().count();
    match host::clipboard::write_text(&text) {
        Ok(()) => app.update(AppEvent::StatusNoticeShown(format!(
            "Copied {char_count} chars to clipboard"
        ))),
        Err(error) => app.update(AppEvent::FailureReported(error)),
    }
}

fn update_pointer_hover(app: &mut App, area: ratatui::layout::Rect, column: u16, row: u16) {
    let target = frame::input_pointer_target_at(app, area, column, row);
    app.update_pointer_hover(target);
}

fn draw_terminal(
    terminal: &mut terminal::TerminalSession,
    app: &App,
) -> Result<(), std::io::Error> {
    terminal.set_mouse_mode(app.mouse_mode())?;
    terminal.draw(|terminal_frame| frame::draw(terminal_frame, app))
}

fn poll_keymap_resource(
    resource: &mut Option<KeymapResource>,
    app: &mut App,
    now: Instant,
) -> bool {
    let Some(resource) = resource else {
        return false;
    };
    match resource.poll(now, &mut app.app_keymap) {
        KeymapResourcePoll::Unchanged => false,
        KeymapResourcePoll::Updated => {
            for diagnostic in resource.diagnostics().to_vec() {
                app.report_keybinding_diagnostic(diagnostic);
            }
            true
        }
        KeymapResourcePoll::Rejected(diagnostic) => {
            app.report_keybinding_diagnostic(diagnostic);
            true
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
            | AppCommand::OpenKeymapPane
            | AppCommand::OpenStatusLinePane
            | AppCommand::EditKeymap(_)
            | AppCommand::EditStatusLine(_)
            | AppCommand::ExportTranscript { .. }
            | AppCommand::ReadClipboardImage
    )
}

#[derive(Default)]
struct ServerRefresh {
    connectors: bool,
    sessions: bool,
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
                let envelope = *request;
                let turn_id = envelope.turn_id;
                let request_id = envelope.interaction.request_id;
                match envelope.interaction.request {
                    zeta_protocol::AgentRequest::Approval { request } => {
                        app.update(AppEvent::ApprovalRequested(Approval::open(
                            turn_id, request_id, request,
                        )));
                    }
                    zeta_protocol::AgentRequest::UserInput { request } => {
                        match Query::open(turn_id, request_id, request) {
                            Ok(query) => app.update(AppEvent::QueryRequested(query)),
                            Err(error) => app.update(AppEvent::FailureReported(error)),
                        }
                    }
                    zeta_protocol::AgentRequest::DynamicTool { .. } => {
                        app.update(AppEvent::FailureReported(
                            "dynamic Tool request is not supported by this TUI".into(),
                        ))
                    }
                }
            }
            ServerRefresh::default()
        }
        client::ClientEvent::SkillsChanged => ServerRefresh {
            skills: true,
            ..ServerRefresh::default()
        },
        client::ClientEvent::ConnectorsChanged => ServerRefresh {
            connectors: app.connector_pane_open(),
            ..ServerRefresh::default()
        },
        client::ClientEvent::PackageSourcesChanged => ServerRefresh {
            connectors: app.connector_pane_open(),
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
        client::ClientEvent::SessionChanged(_) => ServerRefresh {
            sessions: true,
            ..ServerRefresh::default()
        },
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
            match thread_subscription.classify_transcript_update(&update) {
                TranscriptUpdateDisposition::Ignore => ServerRefresh::default(),
                TranscriptUpdateDisposition::Apply => {
                    app.update(AppEvent::ThreadTranscriptUpdateReceived(update));
                    ServerRefresh::default()
                }
                TranscriptUpdateDisposition::RefreshSnapshot => ServerRefresh {
                    thread: true,
                    ..ServerRefresh::default()
                },
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
