use super::*;

pub(super) fn with_shell_presentation_model<R>(
    app: &mut NativeApp,
    window_control_insets: WindowControlInsets,
    operation: impl FnOnce(
        ShellPresentationModel<'_>,
        &mut TextInputLayoutEngine,
        &mut dyn zui::ui::AnimationBinding,
    ) -> R,
) -> R {
    let NativeApp {
        palette,
        retained_runtime,
        workbench_host,
        pane_view_states,
        active_pane: _,
        terminal_pane_resize,
        terminal_scroll,
        terminal_selection,
        workspace_surface,
        thread_projection,
        thread_timeline_scroll,
        workspace_context,
        composer,
        session_search,
        caret_blink,
        ui_dispatch,
        tab_container,
        inspector_part,
        terminal_workspace,
        workspace_pane_host,
        file_editor_host,
        file_editor_input,
        file_editor_search,
        language_service,
        code_editor_style,
        session_context_menu,
        git_branch_context_menu,
        workspace_path_picker,
        remote_connection_picker,
        remote_connection_manager,
        remote_tunnel_manager,
        keybindings,
        keyboard_shortcuts,
        language_server_settings,
        settings_section,
        theme_scheme,
        theme_follows_system,
        cursor_position,
        keybindings_resource,
        text_layout,
        ..
    } = app;
    let workbench = workbench_host.workbench();
    let pane_host = workbench_host.pane_host();
    let file_editor_diagnostics = language_service.active_editor_diagnostics(file_editor_host);
    let language_hover = language_service.active_hover(file_editor_host);
    let language_completions = language_service.active_completions(file_editor_host);
    let language_server_runtime_state =
        language_service.server_state(language_server_settings.selected_server().server_id());
    let active_tab_input = workbench.tab_part().active_tab_key();
    let pane_group = active_tab_input.and_then(|key| workbench.pane_part(key));
    let active_pane = active_tab_input.and_then(|tab_key| {
        let layout = workbench.pane_part(tab_key)?;
        pane_host.mount(
            &PaneHostScope::Tab(tab_key.clone()),
            layout,
            layout.active_group(),
        )
    });
    let active_pane_id = active_pane.map(|mount| mount.pane_id());
    let terminal_panes = pane_group
        .map(|layout| {
            let Some(tab_key) = active_tab_input else {
                return Vec::new();
            };
            layout
                .group_ids()
                .into_iter()
                .filter_map(|pane_id| {
                    let binding = (tab_key.clone(), pane_id);
                    let mount =
                        pane_host.mount(&PaneHostScope::Tab(tab_key.clone()), layout, pane_id)?;
                    let pane_id = mount.pane_id();
                    let kind = mount.kind();
                    let terminal_key = (kind == PaneInputKind::Terminal)
                        .then(|| mount.binding().terminal_key())
                        .flatten();
                    let (scroll_offset, scrollbar_presentation, selection) =
                        if active_pane_id == Some(pane_id) {
                            (
                                terminal_scroll.offset(),
                                terminal_scroll.scrollbar_presentation(),
                                terminal_selection.range(),
                            )
                        } else if let Some(state) = pane_view_states.get(&binding) {
                            (
                                state.scroll.offset(),
                                state.scroll.scrollbar_presentation(),
                                state.selection.range(),
                            )
                        } else {
                            (0, Default::default(), None)
                        };
                    Some(shell_scene::PaneView {
                        pane_id: Some(pane_id),
                        kind,
                        core: terminal_key.and_then(|key| {
                            terminal_workspace.terminal(key).map(TerminalSession::core)
                        }),
                        scroll_offset,
                        scrollbar_presentation,
                        selection,
                    })
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let terminal_key = active_tab_input
        .zip(active_pane_id)
        .and_then(|(tab_key, pane)| {
            pane_host
                .binding(&(PaneHostScope::Tab(tab_key.clone()), pane))
                .and_then(PaneBinding::terminal_key)
        })
        .or_else(|| terminal_workspace.active_key());
    operation(
        ShellPresentationModel {
            palette: *palette,
            terminal: terminal_key
                .and_then(|key| terminal_workspace.terminal(key))
                .map(TerminalSession::core),
            terminal_panes: &terminal_panes,
            pane_group,
            active_pane,
            terminal_pane_resize_split: terminal_pane_resize.as_ref().map(|resize| resize.split_id),
            terminal_scroll_offset: terminal_scroll.offset(),
            terminal_scrollbar_presentation: terminal_scroll.scrollbar_presentation(),
            terminal_selection: terminal_selection.range(),
            workspace_surface: workspace_surface.active(),
            file_editor_host,
            file_editor_prompt: file_editor_input.prompt(),
            file_editor_search,
            file_editor_diagnostics,
            language_hover,
            language_completions,
            completion_selection: file_editor_input.completion_selection(),
            code_editor_style,
            thread_projection,
            thread_timeline_scroll_offset: thread_timeline_scroll.offset(),
            workspace_context,
            composer,
            session_search,
            tab_part: workbench.tab_part(),
            active_tab_input,
            caret_visibility: caret_blink.visibility(),
            dispatch: ui_dispatch,
            tab_container: *tab_container,
            inspector_part: *inspector_part,
            workspace_pane_host,
            session_context_menu: session_context_menu.clone(),
            git_branch_context_menu,
            workspace_path_picker,
            remote_connection_picker,
            remote_connection_manager,
            remote_tunnel_manager,
            keybindings,
            keyboard_shortcuts,
            language_server_settings,
            settings_section: *settings_section,
            language_server_runtime_state,
            keybinding_diagnostics: keybindings_resource.diagnostics(),
            theme_scheme: *theme_scheme,
            theme_follows_system: *theme_follows_system,
            window_control_insets,
            pointer_position: *cursor_position,
        },
        text_layout,
        retained_runtime.animation_registry_mut(),
    )
}
