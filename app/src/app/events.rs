use super::*;

pub(super) fn handle_terminal_event(
    app: &mut NativeApp,
    key: TerminalSessionKey,
    event: TerminalSessionEvent,
) {
    let terminal_exited = matches!(&event, TerminalSessionEvent::Exited(_));
    if app.workbench_host.terminal_workspace.is_pending(key) {
        app.workbench_host
            .terminal_workspace
            .buffer_event_if_pending(key, event);
        session_switch_trace::event(None, "terminal-event-buffered", format_args!("key={key:?}"));
        return;
    }
    if app.active_pane_terminal_key() != Some(key) {
        {
            let Some(terminal) = app.workbench_host.terminal_workspace.terminal_mut(key) else {
                return;
            };
            if let Err(error) = terminal.handle_event(event) {
                eprintln!("could not reply to inactive terminal query: {error}");
            }
        }
        if terminal_exited {
            app.update_terminal_status(key, "Exited");
            app.rebuild_presentation_on_next_redraw();
        }
        return;
    }

    let previous_scroll_limit = app.terminal_scroll_limit();
    let previous_block_status = app
        .active_terminal()
        .and_then(|terminal| terminal.core().block_list().blocks().last())
        .map(|block| block.status());
    let (active_screen, title) = if let Some(terminal) = app.active_terminal_mut() {
        if let Err(error) = terminal.handle_event(event) {
            eprintln!("could not reply to terminal query: {error}");
        }
        (
            terminal.core().active_screen(),
            terminal
                .core()
                .title()
                .unwrap_or(PRODUCT_DISPLAY_NAME)
                .to_owned(),
        )
    } else {
        return;
    };
    if let Some(window) = app.window.as_ref() {
        let _ = window.set_title(&title);
    }
    if terminal_exited {
        app.update_terminal_status(key, "Exited");
    }
    let current_block_status = app
        .active_terminal()
        .and_then(|terminal| terminal.core().block_list().blocks().last())
        .map(|block| block.status());
    if previous_block_status == Some(BlockStatus::Running)
        && current_block_status != Some(BlockStatus::Running)
    {
        if let Some(session) = app.agent_session.as_ref()
            && let Err(error) = session.refresh_git()
        {
            eprintln!("could not refresh Git projection: {error}");
        }
        app.refresh_files_from_app_server();
    }
    if active_screen == ScreenBuffer::Alternate || app.terminal_scroll.offset() == 0 {
        app.terminal_selection.clear();
    }
    let scroll_limit = app.terminal_scroll_limit();
    app.terminal_scroll.preserve_view_after_growth(
        scroll_limit.saturating_sub(previous_scroll_limit),
        scroll_limit,
    );
    app.sync_input_focus();
    app.rebuild_presentation_on_next_redraw();
}
