use super::ActiveConversation;
use super::App;
use super::AppCommand;
use super::AppEvent;
use super::Status;
use super::completion::Completion;
use super::completion::apply_request_completion;
use super::completion::apply_thread_snapshot;
use super::completion::apply_tui_config;
use super::completion::finish_product_command_request;
use super::completion::finish_skill_refresh;
use super::dispatch::execute_product_command;
use super::event_pump::EventPump;
use super::event_pump::RuntimeEvent;
use super::frame;
use super::frame::InputPointerTarget;
use super::redraw::RedrawPriority;
use super::redraw::RedrawScheduler;
use super::requests::RequestLane;
use super::requests::RequestTasks;
use super::requests::request_lane;
use crate::TuiError;
use crate::TuiExit;
use crate::TuiOptions;
use crate::client;
use crate::config;
use crate::dirs;
use crate::host;
use crate::keymap;
use crate::mcp;
use crate::sessions;
use crate::sessions::ResumeOutcome;
use crate::sessions::create_manager_session_and_start;
use crate::sessions::finish_conversation_request;
use crate::skills;
use crate::status as status_line;
use crate::terminal;
use crate::terminal::screen_selection::ClickCount;
use crate::terminal::screen_selection::ScreenSelectionOutcome;
use crate::theme as theme_feature;
use crate::theme::ThemeResource;
use crate::thread::ThreadRequestScope;
use crate::thread::ThreadSubscription;
use crate::thread::ThreadUpdateDisposition;
use crate::thread::TranscriptUpdateDisposition;
use crate::thread::composer::ChatComposerPointerTarget;
use crate::thread::composer::ChatInputCatalog;
use crate::thread::composer::chat_input_catalog_snapshot;
use crate::thread::composer::file_search::FileSearchManager;
use crate::thread::composer::slash_command_registry;
use crate::thread::interaction::approval::Approval;
use crate::thread::interaction::query::Query;
use crate::thread::interrupt_and_read;
use crate::thread::read_older_thread_history;
use crate::thread::read_thread_history;
use crate::thread::resolve_request_and_read;
use crate::thread::rewind;
use crate::thread::start_turn_and_read;
use crate::thread::steer_turn_and_read;
use crate::thread::transcript::batch::TranscriptBatch;
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
        .map(theme_feature::preference)
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
        Ok(config) => apply_tui_config(config, &mut app),
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
    apply_thread_snapshot(&mut app, initial_thread, initial_transcript);
    app.update(AppEvent::SkillDiagnosticsReceived(
        initial_skill_diagnostics,
    ));
    if let Ok(status) = client.git_status() {
        app.update(AppEvent::GitStatusReceived(status));
    }

    let pump = EventPump::start(events)?;
    let mut requests = RequestTasks::default();
    let mut queued_actions = VecDeque::new();
    let mut thread_refresh_requested = false;
    let mut config_refresh_requested = false;
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
            let had_active_turn = app.active_turn().is_some();
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
                        &mut thread_subscription,
                        &mut app,
                    );
                    thread_refresh_requested |= refresh.thread;
                    config_refresh_requested |= refresh.config;
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
                    if app.handle_tick(now) {
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

            for completion in requests.poll() {
                redraw.request(Instant::now(), RedrawPriority::Batched);
                match completion {
                    Ok(completion) => apply_request_completion(
                        completion,
                        &mut conversation,
                        &mut thread_subscription,
                        &mut app,
                    ),
                    Err(error) => app.update(AppEvent::FailureReported(error.to_string())),
                }
            }

            queued_turn_dispatch_requested |= had_active_turn && app.active_turn().is_none();

            let mut action = schedule_action(action, &requests, &mut queued_actions);
            if action.is_none()
                && requests.is_idle(Some(RequestLane::Write))
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
                let action_lane = request_lane(&action);
                match action {
                    AppCommand::ConnectConnectorDeviceOAuth {
                        connector_id,
                        connection_generation,
                    } => {
                        if requests.is_idle(action_lane) {
                            let mut request_client = client.clone();
                            requests.spawn(
                                action_lane,
                                "zeta-tui-connect-device-oauth",
                                move || {
                                    Completion::Presentation(
                                        crate::connectors::connect_device_oauth(
                                            &mut request_client,
                                            connector_id,
                                            connection_generation,
                                        )
                                        .map(AppEvent::ConnectorPickerUpdated)
                                        .map_err(|error| error.to_string()),
                                    )
                                },
                                &mut app,
                            );
                        }
                    }
                    AppCommand::ExecuteProductCommand(invocation) => {
                        if requests.is_idle(action_lane) {
                            let mut request_client = client.clone();
                            let next_conversation = conversation.clone();
                            let next_subscription = thread_subscription.clone();
                            requests.spawn(
                                action_lane,
                                "zeta-tui-product-command",
                                move || {
                                    Completion::ProductCommand(
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
                        if let Some(turn_id) = app.active_turn().cloned()
                            && !matches!(app.status(), Status::Error)
                        {
                            if requests.is_idle(action_lane) {
                                let request_client = client.clone();
                                let scope = thread_request_scope(&conversation);
                                let completion_scope = scope.clone();
                                let history = thread_subscription.history();
                                requests.spawn(
                                    action_lane,
                                    "zeta-tui-interrupt-turn",
                                    move || Completion::TurnInterrupted {
                                        scope: completion_scope,
                                        result: interrupt_and_read(
                                            request_client,
                                            scope,
                                            turn_id,
                                            history,
                                        ),
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
                    AppCommand::ReadClipboardImage => {
                        requests.spawn(
                            action_lane,
                            "zeta-tui-read-clipboard-image",
                            move || {
                                Completion::Presentation(Ok(AppEvent::ClipboardImageRead(
                                    host::clipboard::read_image().map(|image| image.png),
                                )))
                            },
                            &mut app,
                        );
                    }
                    AppCommand::CopyLastResponse => {
                        let response = app
                            .latest_agent_response()
                            .map(str::to_owned)
                            .ok_or_else(|| "there is no Zeta response to copy".to_owned());
                        requests.spawn(
                            action_lane,
                            "zeta-tui-copy-last-response",
                            move || {
                                let event = response
                                    .and_then(|response| {
                                        let char_count = response.chars().count();
                                        host::clipboard::write_text(&response).map(|()| char_count)
                                    })
                                    .map(|char_count| {
                                        AppEvent::TopTipNoticeShown(format!(
                                            "Copied {char_count} chars to clipboard"
                                        ))
                                    })
                                    .unwrap_or_else(|error| {
                                        AppEvent::HostOperationCompleted(Err(error))
                                    });
                                Completion::Presentation(Ok(event))
                            },
                            &mut app,
                        );
                    }
                    AppCommand::OpenConfigEditor => {
                        if requests.is_idle(action_lane) {
                            let mut request_client = client.clone();
                            requests.spawn(
                                action_lane,
                                "zeta-tui-read-config",
                                move || {
                                    Completion::Presentation(
                                        config::read_config_choices(&mut request_client)
                                            .map(AppEvent::ConfigEditorOpened)
                                            .map_err(|error| error.to_string()),
                                    )
                                },
                                &mut app,
                            );
                        }
                    }
                    AppCommand::EditConfig(edit) => {
                        if requests.is_idle(action_lane) {
                            let mut request_client = client.clone();
                            requests.spawn(
                                action_lane,
                                "zeta-tui-set-config",
                                move || {
                                    Completion::Presentation(
                                        config::set_terminal_settings(&mut request_client, edit)
                                            .map(AppEvent::ConfigUpdated)
                                            .map_err(|error| error.to_string()),
                                    )
                                },
                                &mut app,
                            );
                        }
                    }
                    AppCommand::SetProviderApiKey(edit) => {
                        if requests.is_idle(action_lane) {
                            let mut request_client = client.clone();
                            requests.spawn(
                                action_lane,
                                "zeta-tui-set-provider-api-key",
                                move || {
                                    Completion::Presentation(
                                        config::set_provider_api_key(&mut request_client, edit)
                                            .map(|update| AppEvent::ConfigApiKeySaved {
                                                provider: update.provider,
                                                choices: update.choices,
                                            })
                                            .map_err(|error| error.to_string()),
                                    )
                                },
                                &mut app,
                            );
                        }
                    }
                    AppCommand::OpenKeymapEditor => {
                        let mut request_client = client.clone();
                        requests.spawn(
                            action_lane,
                            "zeta-tui-read-keymap",
                            move || {
                                Completion::Presentation(
                                    keymap::read_keymap(&mut request_client)
                                        .map(AppEvent::KeymapEditorOpened),
                                )
                            },
                            &mut app,
                        );
                    }
                    AppCommand::OpenStatusLineEditor => {
                        let mut request_client = client.clone();
                        requests.spawn(
                            action_lane,
                            "zeta-tui-read-status-line",
                            move || {
                                Completion::Presentation(
                                    status_line::read_status_line(&mut request_client)
                                        .map(AppEvent::StatusLineEditorOpened),
                                )
                            },
                            &mut app,
                        );
                    }
                    AppCommand::EditStatusLine(edit) => {
                        let mut request_client = client.clone();
                        requests.spawn(
                            action_lane,
                            "zeta-tui-set-status-line",
                            move || {
                                Completion::Presentation(
                                    status_line::set_status_line(&mut request_client, edit)
                                        .map(AppEvent::StatusLineEditorUpdated),
                                )
                            },
                            &mut app,
                        );
                    }
                    AppCommand::EditKeymap(edit) => {
                        let mut request_client = client.clone();
                        requests.spawn(
                            action_lane,
                            "zeta-tui-set-keymap",
                            move || {
                                Completion::Presentation(
                                    keymap::set_keymap(&mut request_client, edit)
                                        .map(AppEvent::KeymapEditorOpened),
                                )
                            },
                            &mut app,
                        );
                    }
                    AppCommand::ExportTranscript { requested_path } => {
                        let markdown = app.transcript_markdown();
                        let export_root = host_dir_root.clone();
                        requests.spawn(
                            action_lane,
                            "zeta-tui-export-transcript",
                            move || {
                                let result = if markdown.is_empty() {
                                    Err("there is no conversation to export".to_owned())
                                } else {
                                    host::transcript_export::write(
                                        &export_root,
                                        requested_path.as_deref(),
                                        &markdown,
                                    )
                                    .map(|path| {
                                        format!("Exported conversation to {}", path.display())
                                    })
                                };
                                Completion::Presentation(Ok(AppEvent::HostOperationCompleted(
                                    result,
                                )))
                            },
                            &mut app,
                        );
                    }
                    AppCommand::Suspend => terminal.suspend()?,
                    AppCommand::LoadOlderHistory => {
                        if requests.is_idle(action_lane)
                            && let Some(ThreadSnapshotHistory::Before { turn_id, .. }) =
                                thread_subscription.older_history()
                        {
                            let mut request_client = client.clone();
                            let scope = thread_request_scope(&conversation);
                            let session_id = scope.session_id().clone();
                            let thread_id = scope.thread_id().clone();
                            requests.spawn(
                                action_lane,
                                "zeta-tui-load-older-history",
                                move || Completion::ThreadHistoryPage {
                                    scope,
                                    result: read_older_thread_history(
                                        &mut request_client,
                                        &session_id,
                                        &thread_id,
                                        turn_id,
                                    ),
                                },
                                &mut app,
                            );
                        }
                    }
                    AppCommand::OpenThemePicker => {
                        if requests.is_idle(action_lane) {
                            let mut request_client = client.clone();
                            let request_theme_resource = theme_resource.clone();
                            requests.spawn(
                                action_lane,
                                "zeta-tui-open-theme",
                                move || {
                                    Completion::Presentation(
                                        request_client
                                            .read_config()
                                            .map_err(|error| error.to_string())
                                            .and_then(|config| {
                                                request_theme_resource
                                                    .catalog(theme_feature::preference(&config))
                                                    .map(|catalog| {
                                                        AppEvent::ThemePickerOpened(
                                                            theme_feature::theme_choices(&catalog),
                                                        )
                                                    })
                                            }),
                                    )
                                },
                                &mut app,
                            );
                        }
                    }
                    AppCommand::OpenCustomThemePicker => {
                        if requests.is_idle(action_lane) {
                            let mut request_client = client.clone();
                            let request_theme_resource = theme_resource.clone();
                            requests.spawn(
                                action_lane,
                                "zeta-tui-open-custom-theme",
                                move || {
                                    Completion::Presentation(
                                        request_client
                                            .read_config()
                                            .map_err(|error| error.to_string())
                                            .and_then(|config| {
                                                request_theme_resource
                                                    .catalog(theme_feature::preference(&config))
                                                    .map(|catalog| {
                                                        AppEvent::ThemePickerOpened(
                                                            theme_feature::custom_theme_choices(
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
                    AppCommand::OpenRewindPicker => {
                        if requests.is_idle(action_lane) {
                            let mut request_client = client.clone();
                            let session_id = conversation.session_id().clone();
                            let thread_id = conversation.thread_id().clone();
                            requests.spawn(
                                action_lane,
                                "zeta-tui-load-rewind",
                                move || {
                                    Completion::Presentation(
                                        rewind::load_selection(
                                            &mut request_client,
                                            &session_id,
                                            &thread_id,
                                        )
                                        .map(AppEvent::RewindPickerOpened)
                                        .map_err(|error| error.to_string()),
                                    )
                                },
                                &mut app,
                            );
                        }
                    }
                    AppCommand::RemoveDir { path } => {
                        if requests.is_idle(action_lane) {
                            let mut request_client = client.clone();
                            let session_id = conversation.session_id().clone();
                            let event_path = path.clone();
                            requests.spawn(
                                action_lane,
                                "zeta-tui-remove-directory",
                                move || {
                                    Completion::Presentation(
                                        dirs::remove(&mut request_client, &session_id, path)
                                            .map(|choices| AppEvent::DirRemoved {
                                                path: event_path,
                                                choices,
                                            })
                                            .map_err(|error| error.to_string()),
                                    )
                                },
                                &mut app,
                            );
                        }
                    }
                    AppCommand::SetDirPermissions(params) => {
                        if requests.is_idle(action_lane) {
                            let mut request_client = client.clone();
                            requests.spawn(
                                action_lane,
                                "zeta-tui-set-directory-permissions",
                                move || {
                                    Completion::Presentation(
                                        dirs::set_permissions(&mut request_client, params)
                                            .map(AppEvent::DirPermissionsUpdated)
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
                        if requests.is_idle(action_lane) {
                            let mut request_client = client.clone();
                            let mut next_conversation = conversation.clone();
                            let next_subscription = thread_subscription.clone();
                            requests.spawn(
                                action_lane,
                                "zeta-tui-rewind-thread",
                                move || Completion::ConversationChanged {
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
                        if requests.is_idle(action_lane) {
                            let mut request_client = client.clone();
                            let mut next_conversation = conversation.clone();
                            let next_subscription = thread_subscription.clone();
                            requests.spawn(
                                action_lane,
                                "zeta-tui-resume-session",
                                move || match next_conversation.resume_session(
                                    &mut request_client,
                                    &session_id,
                                    preferred_thread_id.as_ref(),
                                ) {
                                    Ok(ResumeOutcome::Changed(change)) => {
                                        Completion::ConversationChanged {
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
                                        Completion::ConversationChanged {
                                            command,
                                            result: Err(
                                                "resume selection did not identify a session"
                                                    .into(),
                                            ),
                                        }
                                    }
                                    Err(error) => Completion::ConversationChanged {
                                        command,
                                        result: Err(error.to_string()),
                                    },
                                },
                                &mut app,
                            );
                        }
                    }
                    AppCommand::ArchiveSessions { session_ids } => {
                        if requests.is_idle(action_lane) {
                            let mut request_client = client.clone();
                            requests.spawn(
                                action_lane,
                                "zeta-tui-archive-sessions",
                                move || {
                                    Completion::Presentation(
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
                        if requests.is_idle(action_lane) {
                            let request = response.identity();
                            let request_client = client.clone();
                            let scope = thread_request_scope(&conversation);
                            let completion_scope = scope.clone();
                            let history = thread_subscription.history();
                            requests.spawn(
                                action_lane,
                                "zeta-tui-resolve-thread-request",
                                move || Completion::ThreadRequestResolved {
                                    scope: completion_scope,
                                    request,
                                    result: resolve_request_and_read(
                                        request_client,
                                        scope,
                                        response,
                                        history,
                                    ),
                                },
                                &mut app,
                            );
                        }
                    }
                    AppCommand::CreateSessionAndEnter { submission } => {
                        if requests.is_idle(action_lane) {
                            let request_client = client.clone();
                            let next_conversation = conversation.clone();
                            let next_subscription = thread_subscription.clone();
                            let approval_mode = app.approval_mode();
                            requests.spawn(
                                action_lane,
                                "zeta-tui-create-manager-session",
                                move || {
                                    Completion::ManagerSessionCreated(
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
                        if requests.is_idle(action_lane) {
                            let mut request_client = client.clone();
                            let mut next_conversation = conversation.clone();
                            let next_subscription = thread_subscription.clone();
                            requests.spawn(
                                action_lane,
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
                                    Completion::ThreadChanged(result)
                                },
                                &mut app,
                            );
                        }
                    }
                    AppCommand::DisconnectConnector { connector_id } => {
                        if requests.is_idle(action_lane) {
                            let mut request_client = client.clone();
                            requests.spawn(
                                action_lane,
                                "zeta-tui-disconnect-connector",
                                move || {
                                    Completion::Presentation(
                                        crate::connectors::disconnect(
                                            &mut request_client,
                                            connector_id,
                                        )
                                        .map(AppEvent::ConnectorPickerUpdated)
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
                        if requests.is_idle(action_lane) {
                            let mut request_client = client.clone();
                            requests.spawn(
                                action_lane,
                                "zeta-tui-set-mcp-enablement",
                                move || {
                                    Completion::Presentation(
                                        mcp::set_enablement(
                                            &mut request_client,
                                            server_id,
                                            enablement,
                                        )
                                        .map(AppEvent::McpSettingsUpdated)
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
                        if requests.is_idle(action_lane) {
                            let mut request_client = client.clone();
                            requests.spawn(
                                action_lane,
                                "zeta-tui-set-preferred-model",
                                move || Completion::PreferredModelUpdated {
                                    command,
                                    result: crate::models::set_preferred_model(
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
                                if requests.is_idle(action_lane) {
                                    let mut request_client = client.clone();
                                    requests.spawn(
                                        action_lane,
                                        "zeta-tui-set-theme",
                                        move || Completion::ThemeUpdated {
                                            command,
                                            label: selection.label,
                                            theme: selection.theme,
                                            result: theme_feature::set_preference(
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
                        if requests.is_idle(action_lane) {
                            let mut request_client = client.clone();
                            let skill_session_id = conversation.session_id().clone();
                            requests.spawn(
                                action_lane,
                                "zeta-tui-set-skill-enablement",
                                move || {
                                    Completion::Presentation(
                                        skills::set_enablement(
                                            &mut request_client,
                                            &skill_session_id,
                                            skill_id,
                                            enablement,
                                        )
                                        .map(AppEvent::SkillSettingsUpdated)
                                        .map_err(|error| error.to_string()),
                                    )
                                },
                                &mut app,
                            );
                        }
                    }
                    AppCommand::CycleNextApprovalMode => {
                        app.cycle_next_approval_mode(Instant::now());
                    }
                    AppCommand::SubmitTurn { submission } => {
                        if requests.is_idle(action_lane) {
                            let request_client = client.clone();
                            let scope = thread_request_scope(&conversation);
                            let completion_scope = scope.clone();
                            let approval_mode = app.approval_mode();
                            let history = thread_subscription.history();
                            requests.spawn(
                                action_lane,
                                "zeta-tui-start-turn",
                                move || Completion::TurnStarted {
                                    scope: completion_scope,
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
                    AppCommand::SubmitQueuedTurn {
                        queue_id,
                        submission,
                    } => {
                        if requests.is_idle(action_lane) {
                            let request_client = client.clone();
                            let scope = thread_request_scope(&conversation);
                            let completion_scope = scope.clone();
                            let approval_mode = app.approval_mode();
                            let history = thread_subscription.history();
                            requests.spawn(
                                action_lane,
                                "zeta-tui-start-queued-turn",
                                move || Completion::QueuedTurnStarted {
                                    scope: completion_scope,
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
                        if requests.is_idle(action_lane) {
                            if matches!(app.status(), Status::Working) && !app.steers_active_turn()
                            {
                                queued_actions.push_front(AppCommand::SteerTurn {
                                    steer_id,
                                    submission,
                                });
                            } else if let Some(turn_id) = app.active_turn().cloned() {
                                let request_client = client.clone();
                                let scope = thread_request_scope(&conversation);
                                let completion_scope = scope.clone();
                                let history = thread_subscription.history();
                                requests.spawn(
                                    action_lane,
                                    "zeta-tui-steer-turn",
                                    move || Completion::TurnSteered {
                                        scope: completion_scope,
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
            if requests.is_idle(Some(RequestLane::Read)) && config_refresh_requested {
                let mut request_client = client.clone();
                requests.spawn(
                    Some(RequestLane::Read),
                    "zeta-tui-refresh-config",
                    move || {
                        Completion::ConfigRefreshed(
                            request_client
                                .read_config()
                                .map_err(|error| error.to_string()),
                        )
                    },
                    &mut app,
                );
                if !requests.is_idle(Some(RequestLane::Read)) {
                    config_refresh_requested = false;
                }
            }
            if requests.is_idle(Some(RequestLane::Read)) && thread_refresh_requested {
                let mut request_client = client.clone();
                let scope = thread_request_scope(&conversation);
                let session_id = scope.session_id().clone();
                let thread_id = scope.thread_id().clone();
                let history = thread_subscription.history();
                requests.spawn(
                    Some(RequestLane::Read),
                    "zeta-tui-refresh-thread",
                    move || Completion::ThreadRefreshed {
                        scope,
                        result: read_thread_history(
                            &mut request_client,
                            &session_id,
                            &thread_id,
                            history,
                        ),
                    },
                    &mut app,
                );
                if !requests.is_idle(Some(RequestLane::Read)) {
                    thread_refresh_requested = false;
                }
            }
            if requests.is_idle(Some(RequestLane::Read)) && skills_refresh_requested {
                let request_client = client.clone();
                let server_slash_commands = server_slash_commands.clone();
                let skills_session_id = conversation.session_id().clone();
                requests.spawn(
                    Some(RequestLane::Read),
                    "zeta-tui-refresh-skills",
                    move || {
                        Completion::SkillsRefreshed(
                            skills::refresh(request_client, skills_session_id, plugins_enabled)
                                .and_then(|refresh| {
                                    finish_skill_refresh(refresh, &server_slash_commands)
                                }),
                        )
                    },
                    &mut app,
                );
                if !requests.is_idle(Some(RequestLane::Read)) {
                    skills_refresh_requested = false;
                }
            }
            if requests.is_idle(Some(RequestLane::Read)) && sessions_refresh_requested {
                let mut request_client = client.clone();
                requests.spawn(
                    Some(RequestLane::Read),
                    "zeta-tui-refresh-sessions",
                    move || {
                        Completion::Presentation(
                            sessions::load_catalog(&mut request_client)
                                .map(AppEvent::SessionCatalogReceived)
                                .map_err(|error| error.to_string()),
                        )
                    },
                    &mut app,
                );
                if !requests.is_idle(Some(RequestLane::Read)) {
                    sessions_refresh_requested = false;
                }
            }
            if requests.is_idle(Some(RequestLane::Read)) && connectors_refresh_requested {
                let mut request_client = client.clone();
                requests.spawn(
                    Some(RequestLane::Read),
                    "zeta-tui-refresh-connectors",
                    move || {
                        Completion::Presentation(
                            crate::connectors::load_selection(&mut request_client)
                                .map(AppEvent::ConnectorPickerUpdated)
                                .map_err(|error| error.to_string()),
                        )
                    },
                    &mut app,
                );
                if !requests.is_idle(Some(RequestLane::Read)) {
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
        InputPointerTarget::InputSurface(
            crate::app::input_surface::InputSurfacePointerTarget::Tab(index),
        ) => {
            app.select_tab(index);
            None
        }
        InputPointerTarget::InputSurface(
            crate::app::input_surface::InputSurfacePointerTarget::Search,
        ) => {
            app.focus_composer_search();
            None
        }
        InputPointerTarget::InputSurface(
            crate::app::input_surface::InputSurfacePointerTarget::Item(index),
        ) => app.activate_visible_item(index),
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
    range: crate::terminal::screen_selection::ScreenSelectionRange,
) {
    app.select_screen_range(range);
    let Some(text) = terminal.selected_text(range) else {
        return;
    };
    let char_count = text.chars().count();
    match host::clipboard::write_text(&text) {
        Ok(()) => app.update(AppEvent::TopTipNoticeShown(format!(
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

fn schedule_action(
    action: Option<AppCommand>,
    requests: &RequestTasks,
    queued: &mut VecDeque<AppCommand>,
) -> Option<AppCommand> {
    if let Some(action) = action {
        queued.push_back(action);
    }
    let runnable = queued
        .iter()
        .position(|action| requests.is_idle(request_lane(action)))?;
    queued.remove(runnable)
}

#[derive(Default)]
struct ServerRefresh {
    config: bool,
    connectors: bool,
    sessions: bool,
    thread: bool,
    skills: bool,
}

fn refresh_server_event(
    event: client::ClientEvent,
    conversation: &mut ActiveConversation,
    thread_subscription: &mut ThreadSubscription,
    app: &mut App,
) -> ServerRefresh {
    match event {
        client::ClientEvent::ConfigChanged => ServerRefresh {
            config: true,
            ..ServerRefresh::default()
        },
        client::ClientEvent::AgentRequest(request) => {
            if request.session_id == *conversation.session_id()
                && request.thread_id == *conversation.thread_id()
            {
                app.set_active_turn(request.turn_id.clone());
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
            connectors: app.connector_picker_open(),
            ..ServerRefresh::default()
        },
        client::ClientEvent::PackageSourcesChanged => ServerRefresh {
            connectors: app.connector_picker_open(),
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
