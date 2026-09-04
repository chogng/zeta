use super::ActiveConversation;
use super::App;
use super::AppCommand;
use super::completion::apply_thread_snapshot;
use super::completion::apply_tui_config;
use super::driver::AppDriver;
use super::driver::AppDriverResources;
use super::driver::CommandEffect;
use super::event_pump::EventPump;
use super::event_pump::RuntimeEvent;
use super::frame;
use super::frame::InputPointerTarget;
use super::redraw::RedrawPriority;
use super::redraw::RedrawScheduler;
use crate::AppServerProcess;
use crate::TuiError;
use crate::TuiExit;
use crate::TuiOptions;
use crate::client;
use crate::host;
use crate::host::Command as HostCommand;
use crate::host::Event as HostEvent;
use crate::host::process_resources::ProcessResourceDemand;
use crate::host::process_resources::ProcessResourceTargets;
use crate::sessions;
use crate::sessions::Event as SessionEvent;
use crate::skills::Event as SkillEvent;
use crate::status::Event as StatusEvent;
use crate::terminal;
use crate::terminal::screen_selection::ClickCount;
use crate::terminal::screen_selection::ScreenSelectionOutcome;
use crate::theme as theme_feature;
use crate::theme::Event as ThemeEvent;
use crate::theme::ThemeResource;
use crate::thread::Event as ThreadEvent;
use crate::thread::ThreadSubscription;
use crate::thread::composer::ChatComposerPointerTarget;
use crate::thread::composer::ChatInputCatalog;
use crate::thread::composer::chat_input_catalog_snapshot;
use crate::thread::composer::file_search::FileSearchManager;
use crate::thread::composer::slash_command_registry;
use crate::thread::transcript::TranscriptScrollDirection;
use crate::thread::transcript::batch::TranscriptBatch;
use crossterm::event::Event;
use crossterm::event::KeyEventKind;
use crossterm::event::MouseButton;
use crossterm::event::MouseEventKind;
use std::time::Instant;
use zeta_app_server_client::AppServerSession;
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
    let startup_context = options.startup_context();
    let TuiOptions {
        thread_title,
        display_dir_root,
        host_dir_root,
        host_file_search_root,
        theme_root,
        app_server_process,
        recovery,
        ..
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
    let (thread_subscription, initial_thread, initial_transcript) = ThreadSubscription::start(
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
    let file_search = host_file_search_root.map(FileSearchManager::new);
    let mut app = App::for_dir_with_input_catalog_and_startup_context(
        &display_dir_root,
        input_catalog,
        startup_context,
    );
    let initial_config = client.read_config();
    let initial_model_catalog = client.list_models().ok();
    let theme_preference = initial_config
        .as_ref()
        .map(theme_feature::preference)
        .unwrap_or("system");
    match theme_resource.load(theme_preference) {
        Ok(loaded) => {
            for diagnostic in loaded.diagnostics {
                eprintln!("theme: {diagnostic}");
            }
            app.update(ThemeEvent::RenderChanged(loaded.theme));
        }
        Err(error) => app.update(ThreadEvent::FailureReported(error)),
    }
    match initial_config {
        Ok(config) => apply_tui_config(config, initial_model_catalog.as_ref(), &mut app),
        Err(error) => app.update(ThreadEvent::FailureReported(format!(
            "could not read server configuration: {error}"
        ))),
    }
    match sessions::load_catalog(&mut client) {
        Ok(catalog) => app.update(SessionEvent::CatalogReceived(catalog)),
        Err(error) => app.update(ThreadEvent::FailureReported(format!(
            "could not load Sessions: {error}"
        ))),
    }
    apply_thread_snapshot(&mut app, initial_thread, initial_transcript);
    app.update(SkillEvent::DiagnosticsReceived(initial_skill_diagnostics));
    if let Ok(status) = client.git_status() {
        app.update(StatusEvent::GitStatusReceived(status));
    }
    if app.request_git_text_diff()
        && let Ok(result) = client.git_text_diff()
    {
        app.update(StatusEvent::GitTextDiffReceived {
            status: result.status,
            statistics: result.statistics,
        });
    }

    let resource_targets = match app_server_process {
        AppServerProcess::Local(process_id) => ProcessResourceTargets::TuiAndAppServer(process_id),
        AppServerProcess::IncludedInTui | AppServerProcess::Remote => ProcessResourceTargets::Tui,
    };
    let mut driver = AppDriver::new(
        app,
        client,
        conversation,
        thread_subscription,
        AppDriverResources {
            file_search,
            host_dir_root,
            theme_resource,
            server_slash_commands,
            plugins_enabled,
        },
    );
    let mut pump = EventPump::start(events, resource_targets)?;
    let mut redraw = RedrawScheduler::default();
    let mut process_resource_demand = ProcessResourceDemand::Disabled;
    let mut pending_runtime_event = None;
    if let Err(error) = draw_terminal(&mut terminal, driver.app()) {
        let _ = pump.shutdown();
        return Err(error.into());
    }
    let result = (|| {
        loop {
            sync_process_resource_demand(
                &mut pump,
                driver.app_mut(),
                terminal.area()?,
                &mut process_resource_demand,
            );
            let had_active_turn = driver.app().active_turn().is_some();
            let mut runtime_event = match pending_runtime_event.take() {
                Some(event) => event,
                None => match redraw.wait_timeout(Instant::now()) {
                    Some(timeout) => match pump.recv_timeout(timeout)? {
                        Some(event) => event,
                        None => {
                            if redraw.take_due(Instant::now()) {
                                draw_terminal(&mut terminal, driver.app())?;
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
                | RuntimeEvent::ProcessResources(_)
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
            let action =
                match runtime_event {
                    RuntimeEvent::Client(event) => {
                        let event = match super::recovery::continue_or_exit(
                            event,
                            driver.session_id(),
                            driver.thread_id(),
                        ) {
                            Ok(event) => event,
                            Err(exit) => return Ok(exit),
                        };
                        driver.handle_client_event(event);
                        None
                    }
                    RuntimeEvent::TerminationRequested => {
                        return Ok(TuiExit::TerminationRequested);
                    }
                    RuntimeEvent::ProcessResources(reading) => {
                        driver
                            .app_mut()
                            .update(HostEvent::ProcessResourcesSampled(reading));
                        if !matches!(process_resource_demand, ProcessResourceDemand::Disabled) {
                            redraw.request(Instant::now(), RedrawPriority::Batched);
                        }
                        None
                    }
                    RuntimeEvent::Terminal(terminal::TerminalEvent::Failed(error)) => {
                        return Err(error.into());
                    }
                    RuntimeEvent::Terminal(terminal::TerminalEvent::Tick) => {
                        let now = Instant::now();
                        if driver.app_mut().handle_tick(now) {
                            redraw.request(now, RedrawPriority::Batched);
                        }
                        None
                    }
                    RuntimeEvent::Terminal(terminal::TerminalEvent::Input(event)) => {
                        match event {
                            Event::FocusGained => {
                                Some(HostCommand::RefreshClipboardImageAvailability.into())
                            }
                            Event::Key(key) if key.kind != KeyEventKind::Release => {
                                driver.app_mut().handle_key_in_area(key, terminal.area()?)
                            }
                            Event::Mouse(mouse)
                                if mouse.kind == MouseEventKind::Down(MouseButton::Left) =>
                            {
                                let terminal_area = terminal.area()?;
                                let target = frame::input_pointer_target_at(
                                    driver.app(),
                                    terminal_area,
                                    mouse.column,
                                    mouse.row,
                                );
                                driver.app_mut().update_pointer_pressed(target);
                                driver.app_mut().begin_screen_selection(
                                    ratatui::layout::Position::new(mouse.column, mouse.row),
                                );
                                None
                            }
                            Event::Mouse(mouse)
                                if mouse.kind == MouseEventKind::Drag(MouseButton::Left) =>
                            {
                                driver.app_mut().clear_pointer_pressed();
                                driver.app_mut().drag_screen_selection(
                                    ratatui::layout::Position::new(mouse.column, mouse.row),
                                );
                                None
                            }
                            Event::Mouse(mouse)
                                if mouse.kind == MouseEventKind::Up(MouseButton::Left) =>
                            {
                                finish_pointer_gesture(
                                    driver.app_mut(),
                                    &terminal,
                                    ratatui::layout::Position::new(mouse.column, mouse.row),
                                )?
                            }
                            Event::Mouse(mouse) if mouse.kind == MouseEventKind::Moved => {
                                let terminal_area = terminal.area()?;
                                update_pointer_hover(
                                    driver.app_mut(),
                                    terminal_area,
                                    mouse.column,
                                    mouse.row,
                                );
                                None
                            }
                            Event::Mouse(mouse)
                                if matches!(
                                    mouse.kind,
                                    MouseEventKind::ScrollUp | MouseEventKind::ScrollDown
                                ) =>
                            {
                                let terminal_area = terminal.area()?;
                                let direction = if mouse.kind == MouseEventKind::ScrollUp {
                                    TranscriptScrollDirection::Up
                                } else {
                                    TranscriptScrollDirection::Down
                                };
                                scroll_pointer_item(
                                    driver.app_mut(),
                                    terminal_area,
                                    mouse.column,
                                    mouse.row,
                                    direction,
                                )
                            }
                            Event::Paste(text) => {
                                driver.app_mut().handle_paste(text);
                                None
                            }
                            Event::Resize(_, _) => {
                                driver.app_mut().clear_pointer_interaction();
                                None
                            }
                            _ => None,
                        }
                    }
                };

            if driver.poll_request_completions() {
                redraw.request(Instant::now(), RedrawPriority::Batched);
            }

            let command = driver.next_command(action, had_active_turn);
            if driver.poll_file_search() {
                redraw.request(Instant::now(), RedrawPriority::Batched);
            }
            if let Some(command) = command {
                match driver.execute(command) {
                    CommandEffect::None => {}
                    CommandEffect::Quit => return Ok(TuiExit::UserRequested),
                    CommandEffect::Suspend => terminal.suspend()?,
                }
            }
            driver.schedule_refreshes();
            sync_process_resource_demand(
                &mut pump,
                driver.app_mut(),
                terminal.area()?,
                &mut process_resource_demand,
            );
            if redraw.take_due(Instant::now()) {
                draw_terminal(&mut terminal, driver.app())?;
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

fn sync_process_resource_demand(
    pump: &mut EventPump,
    app: &mut App,
    terminal_area: ratatui::layout::Rect,
    current: &mut ProcessResourceDemand,
) {
    let next = frame::process_resource_demand(app, terminal_area);
    if next == *current {
        return;
    }
    let request = pump.set_process_resource_demand(next);
    app.apply_process_resource_request(request);
    *current = next;
}

fn activate_pointer_item(
    app: &mut App,
    area: ratatui::layout::Rect,
    column: u16,
    row: u16,
) -> Option<AppCommand> {
    let target = frame::input_pointer_target_at(app, area, column, row)?;
    match target {
        InputPointerTarget::CommandPanel(
            crate::app::command_panel::CommandPanelPointerTarget::Tab(index),
        ) => {
            app.select_tab(index);
            None
        }
        InputPointerTarget::CommandPanel(
            crate::app::command_panel::CommandPanelPointerTarget::Search,
        ) => {
            app.focus_composer_search();
            None
        }
        InputPointerTarget::CommandPanel(
            crate::app::command_panel::CommandPanelPointerTarget::Item(index),
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
        InputPointerTarget::Queue(queue_id) => {
            app.activate_queue_pointer_target(queue_id);
            None
        }
        InputPointerTarget::TranscriptJumpToBottom => {
            app.follow_latest_transcript();
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

fn scroll_pointer_item(
    app: &mut App,
    area: ratatui::layout::Rect,
    column: u16,
    row: u16,
    direction: TranscriptScrollDirection,
) -> Option<AppCommand> {
    if app.scroll_session_manager(direction == TranscriptScrollDirection::Up) {
        return None;
    }
    if !frame::transcript_contains(app, area, column, row) {
        return None;
    }
    app.navigate_transcript(direction, area)
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
        Ok(()) => app.update(HostEvent::TopTipNoticeShown(format!(
            "Copied {char_count} chars to clipboard"
        ))),
        Err(error) => app.update(ThreadEvent::FailureReported(error)),
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

#[cfg(test)]
#[path = "event_loop_tests.rs"]
mod tests;
