use std::path::Path;
use std::path::PathBuf;

use zeta_app_server_protocol::protocol::fs::FsChanged;
use zeta_protocol::Session;
use zeta_text_file::TextFileSaveRequest;

use crate::NativeApp;
use crate::composer_host::composer_model_options;
use crate::composer_host::synchronize_composer_classifier;
use crate::composer_host::update_composer_classifier;
use crate::session::session_switch_trace;
use crate::session::session_switch_trace::SwitchId;
use crate::thread_projection::ThreadProjectionUpdate;
use crate::workspace_pane_host::WorkspacePaneView;
use zeta_workbench::PaneInput;
use zeta_workbench::TabInput;
use zeta_workbench::TabInputChange;
use zeta_workbench::TabInputKey;
use zeta_workbench::TabInputMetadata;

pub(crate) use zeta_agent_session::AgentSession;
pub(crate) use zeta_agent_session::AgentSessionEvent;
pub(crate) use zeta_agent_session::WorkspaceSwitchResult;

impl NativeApp {
    pub(crate) fn add_session(&mut self) {
        let Some(session) = self.agent_session.as_ref() else {
            eprintln!("could not create session: App Server session is unavailable");
            return;
        };
        session_switch_trace::event(
            None,
            "session-create-request",
            format_args!("source=add-session"),
        );
        if let Err(error) = session.create_session() {
            eprintln!("could not create session: {error}");
        }
    }

    pub(crate) fn activate_session_tab(&mut self, index: usize) {
        self.activate_session_tab_at(index, false);
    }

    /// Reconnects the App Server worker after Workbench has selected a replacement for a closed
    /// Session tab. The Workbench selection is already updated, so the normal already-selected
    /// guard must be bypassed once.
    pub(crate) fn activate_session_tab_after_close(&mut self, tab_key: &TabInputKey) {
        let Some(index) =
            (0..self.workbench.workbench().tab_part().session_count()).find(|index| {
                self.workbench
                    .workbench()
                    .tab_part()
                    .session_input_at(*index)
                    .is_some_and(|input| input.key() == tab_key)
            })
        else {
            return;
        };
        self.activate_session_tab_at(index, true);
    }

    fn activate_session_tab_at(&mut self, index: usize, force: bool) {
        let switch_id = session_switch_trace::SwitchId::next();
        let Some(tab) = self
            .workbench
            .workbench()
            .tab_part()
            .session_input_at(index)
        else {
            session_switch_trace::event(
                Some(switch_id),
                "activation-rejected",
                format_args!("reason=missing-tab index={index}"),
            );
            return;
        };
        let Some(session_id) = tab.session_id().cloned() else {
            session_switch_trace::event(
                Some(switch_id),
                "activation-rejected",
                format_args!("reason=non-session-tab index={index}"),
            );
            return;
        };
        let target_workspace_root = tab.workspace_root().map(Path::to_path_buf);
        session_switch_trace::event(
            Some(switch_id),
            "activation-request",
            format_args!("index={index} session_id={session_id}"),
        );
        if !force && self.workbench.workbench().tab_part().selected_session() == Some(&session_id) {
            if self.workbench.workbench().tab_part().is_settings() {
                self.activate_session_workbench_tab();
                self.rebuild_presentation_on_next_redraw();
            } else {
                session_switch_trace::event(
                    Some(switch_id),
                    "activation-rejected",
                    format_args!("reason=already-selected"),
                );
            }
            return;
        }
        let switches_workspace = target_workspace_root
            .as_deref()
            .is_some_and(|target| target != self.workspace_context.working_directory());
        if switches_workspace
            && self.file_editor_host.request_workspace_replace()
                == crate::file_editor_host::FileEditorCloseRequest::NeedsConfirmation
        {
            session_switch_trace::event(
                Some(switch_id),
                "activation-rejected",
                format_args!("reason=unsaved-workspace-file"),
            );
            eprintln!("could not open Session Workspace while the active file has unsaved changes");
            return;
        }
        let was_terminal = self.workspace_surface.is_terminal();
        let ensured = {
            let _trace = session_switch_trace::Span::new(Some(switch_id), "ensure-terminal");
            self.ensure_terminal_for_session(&session_id)
        };
        if !ensured {
            session_switch_trace::event(
                Some(switch_id),
                "activation-rejected",
                format_args!("reason=terminal-ensure-failed"),
            );
            return;
        }
        let Some(session) = self.agent_session.as_ref() else {
            session_switch_trace::event(
                Some(switch_id),
                "activation-rejected",
                format_args!("reason=agent-session-unavailable"),
            );
            return;
        };
        let workspace_switch = match session.activate_session(
            session_id.clone(),
            zeta_agent_session::SessionSwitchId::new(switch_id.get()),
        ) {
            Ok(workspace_switch) => workspace_switch,
            Err(error) => {
                session_switch_trace::event(
                    Some(switch_id),
                    "activation-rejected",
                    format_args!("reason=agent-command-queue error={error}"),
                );
                eprintln!("could not activate session: {error}");
                return;
            }
        };
        if let Some(result) = workspace_switch
            && !self.apply_workspace_switch_result(result)
        {
            return;
        }
        self.workbench.workbench_mut().activate_session(&session_id);
        self.activate_session_workbench_tab();
        let terminal_activated = self.activate_terminal_for_session(&session_id);
        if !was_terminal {
            let _ = self.bind_agent_pane();
        }
        session_switch_trace::event(
            Some(switch_id),
            "local-terminal-activation",
            format_args!("success={terminal_activated}"),
        );
        {
            let _trace = session_switch_trace::Span::new(Some(switch_id), "local-ui-invalidation");
            self.rebuild_presentation_on_next_redraw();
        }
        session_switch_trace::event(
            Some(switch_id),
            "local-activation-visible",
            format_args!("selected_session={session_id}"),
        );
    }

    fn upsert_session_tab(&mut self, session: &Session) {
        let workspace = self.workspace_context.working_directory_label().to_owned();
        let result = self.workbench.workbench_mut().upsert_session_input(
            session_tab_input(session, &workspace),
            PaneInput::terminal(session.session_id.clone()),
        );
        let (label, input_key) = match result {
            TabInputChange::Added(input_key) => ("session-tab-added", input_key),
            TabInputChange::Updated(input_key) => ("session-tab-updated", input_key),
        };
        session_switch_trace::event(
            None,
            label,
            format_args!(
                "session_id={} input={input_key:?} tab_count={}",
                session.session_id,
                self.workbench.workbench().tab_part().session_count()
            ),
        );
    }

    fn upsert_session_catalog(&mut self, sessions: &[Session]) {
        let workspace = self.workspace_context.working_directory_label().to_owned();
        for session in sessions {
            self.workbench.workbench_mut().upsert_catalog_session_input(
                session_tab_input(session, &workspace),
                PaneInput::terminal(session.session_id.clone()),
            );
        }
    }

    pub(crate) fn handle_agent_session_event(&mut self, event: AgentSessionEvent) {
        let previous_line_count = crate::thread_timeline::line_count(&self.thread_projection);
        let workspace_may_have_changed = matches!(
            &event,
            AgentSessionEvent::Update(update)
                if matches!(
                    &update.update,
                    zeta_protocol::ThreadUpdate::Committed {
                        event: zeta_protocol::ThreadEvent::ItemCompleted {
                            item: zeta_protocol::ThreadItem::ToolResult { .. },
                            ..
                        }
                    }
                )
        );
        match event {
            AgentSessionEvent::Catalog {
                slash_commands,
                models,
            } => {
                if let Err(error) = self
                    .composer
                    .interaction_mut()
                    .set_catalog(slash_commands, composer_model_options(models))
                {
                    eprintln!("could not install Slash Commands catalog: {error}");
                }
            }
            AgentSessionEvent::Configuration(configuration) => {
                self.language_server_settings.synchronize(&configuration);
                self.language_service
                    .apply_configuration(&configuration, &self.file_editor_host);
            }
            AgentSessionEvent::SessionCatalog(sessions) => {
                self.upsert_session_catalog(&sessions);
            }
            AgentSessionEvent::Snapshot {
                session,
                thread,
                switch_id,
            } => {
                let switch_id = switch_id.map(|switch_id| SwitchId::new(switch_id.get()));
                session_switch_trace::event(
                    switch_id,
                    "snapshot-received",
                    format_args!(
                        "session_id={} thread_id={}",
                        session.session_id, thread.thread_id
                    ),
                );
                self.upsert_session_tab(&session);
                self.ensure_terminal_for_session(&session.session_id);
                self.activate_terminal_for_session(&session.session_id);
                synchronize_composer_classifier(&mut self.composer, &thread);
                self.thread_projection.replace_snapshot(thread);
                let active_session = self
                    .active_session_tab_key()
                    .and_then(|key| key.session_id().cloned());
                if active_session.as_ref() == Some(&session.session_id)
                    && !self.workspace_surface.is_terminal()
                {
                    let _ = self.bind_agent_pane();
                }
            }
            AgentSessionEvent::Update(update) => {
                update_composer_classifier(&mut self.composer, &update.update);
                if self.thread_projection.apply_update(*update)
                    == ThreadProjectionUpdate::ResubscribeRequired
                    && let Some(session) = self.agent_session.as_ref()
                    && let Err(error) = session.refresh()
                {
                    eprintln!("could not refresh Agent Thread projection: {error}");
                }
            }
            AgentSessionEvent::GitSnapshot(snapshot) => {
                self.workspace_context
                    .apply_git_projection(snapshot.as_ref());
                self.sync_workspace_pane_repository();
                self.refresh_files_from_app_server();
            }
            AgentSessionEvent::FilesChanged(changed) => {
                if shell_completion_sources_changed(&changed) {
                    self.composer.refresh_shell_workspace();
                }
                self.refresh_files_from_app_server();
                self.refresh_open_files_from_app_server(&changed);
            }
            AgentSessionEvent::Error(error) => {
                eprintln!("Agent session failed: {error}");
            }
            AgentSessionEvent::Closed => {}
        }
        if workspace_may_have_changed {
            if let Some(session) = self.agent_session.as_ref()
                && let Err(error) = session.refresh_git()
            {
                eprintln!("could not refresh Git projection: {error}");
            }
        }
        let line_count = crate::thread_timeline::line_count(&self.thread_projection);
        let limit = self.thread_timeline_scroll_limit();
        self.thread_timeline_scroll
            .preserve_view_after_growth(line_count.saturating_sub(previous_line_count), limit);
        self.thread_timeline_scroll.clamp(limit);
        self.rebuild_presentation_on_next_redraw();
    }
}

fn session_tab_input(session: &Session, workspace_label: &str) -> TabInput {
    let workspace = session
        .workspace
        .as_ref()
        .and_then(|binding| binding.root.file_name())
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .unwrap_or(workspace_label);
    let mut metadata = TabInputMetadata::new(&session.title, workspace).with_status_label("Active");
    if let Some(workspace_root) = session
        .workspace
        .as_ref()
        .map(|binding| binding.root.clone())
    {
        metadata = metadata.with_workspace_root(workspace_root);
    }
    TabInput::session(session.session_id.clone(), metadata)
}

fn shell_completion_sources_changed(changed: &FsChanged) -> bool {
    match changed {
        FsChanged::RescanRequired { .. } => true,
        FsChanged::PathsChanged { paths, .. } => paths.iter().any(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| {
                    matches!(
                        name,
                        "package.json"
                            | "Justfile"
                            | "justfile"
                            | ".justfile"
                            | "Makefile"
                            | "makefile"
                            | "GNUmakefile"
                    )
                })
        }),
    }
}

impl NativeApp {
    pub(crate) fn replace_workspace_pane(&mut self) {
        let pane_kind = self.active_workspace_pane_kind();
        let removed = self
            .workspace_pane_host
            .replace_workspace(&self.workspace_context);
        let view = match pane_kind {
            Some(zeta_workbench::PaneInputKind::Diff) => Some(WorkspacePaneView::Changes),
            Some(zeta_workbench::PaneInputKind::Files) => Some(WorkspacePaneView::Files),
            _ => None,
        };
        if let Some(view) = view {
            self.select_workspace_pane_view(view);
        }
        self.remove_scm_animation_tracks(removed);
    }

    fn sync_workspace_pane_repository(&mut self) {
        let removed = self
            .workspace_pane_host
            .sync_repository(&self.workspace_context);
        self.remove_scm_animation_tracks(removed);
    }

    fn remove_scm_animation_tracks(
        &mut self,
        removed: Vec<zeta_editor::MultiDiffEditorItemIdentity>,
    ) {
        for identity in removed {
            self.retained_runtime
                .animation_registry_mut()
                .remove_element(identity.section_id());
        }
    }

    pub(crate) fn refresh_files_from_app_server(&mut self) {
        let Some(session) = self.agent_session.as_ref() else {
            return;
        };
        match session.read_directory(PathBuf::from(".")) {
            Ok(entries) => self.workspace_pane_host.refresh_files(entries),
            Err(error) => eprintln!("could not read App Server workspace directory: {error}"),
        }
    }

    pub(crate) fn load_file_tree_directory(&mut self, element: zui::ui::ElementId, path: PathBuf) {
        let Some(session) = self.agent_session.as_ref() else {
            return;
        };
        match session.read_directory(path) {
            Ok(entries) => {
                self.workspace_pane_host
                    .complete_file_tree_directory_load(element, entries);
            }
            Err(error) => eprintln!("could not read App Server workspace directory: {error}"),
        }
    }

    pub(crate) fn open_workspace_file(&mut self, path: PathBuf) {
        let Some(session) = self.agent_session.as_ref() else {
            return;
        };
        match session.read_file(path) {
            Ok(snapshot) => {
                self.file_editor_host.open(snapshot);
                self.language_service
                    .synchronize_active(&self.file_editor_host);
                self.file_editor_input.reset_for_document_change();
                self.show_agent_pane();
                self.inspector_part.expand();
                self.workspace_surface.show_editor();
                self.pending_focus = Some(crate::shell_interaction::FILE_EDITOR_DOCUMENT);
                self.rebuild_presentation();
                self.request_redraw();
            }
            Err(error) => eprintln!("could not open App Server workspace file: {error}"),
        }
    }

    pub(crate) fn open_language_definition(
        &mut self,
        target: zeta_language_service::LanguageLocationTarget,
    ) {
        let Some(session) = self.agent_session.as_ref() else {
            return;
        };
        match session.read_file(target.path) {
            Ok(snapshot) => {
                self.file_editor_host.open(snapshot);
                if let Some(position) = definition_editor_position(
                    self.file_editor_host
                        .active()
                        .map(|tab| tab.document().text())
                        .unwrap_or_default(),
                    target.selection_range.start.row,
                    target.selection_range.start.character,
                    target.encoding,
                ) {
                    self.file_editor_host
                        .move_active_caret(position, zeta_editor::CodeEditorSelectionMode::Move);
                }
                self.language_service
                    .synchronize_active(&self.file_editor_host);
                self.file_editor_input.reset_for_document_change();
                self.show_agent_pane();
                self.inspector_part.expand();
                self.workspace_surface.show_editor();
                self.pending_focus = Some(crate::shell_interaction::FILE_EDITOR_DOCUMENT);
                self.rebuild_presentation();
                self.request_redraw();
            }
            Err(error) => eprintln!("could not open language definition: {error}"),
        }
    }

    pub(crate) fn save_active_workspace_file(&mut self) {
        let Some(request) = self.file_editor_host.save_request() else {
            return;
        };
        let _ = self.write_active_workspace_file(request);
    }

    pub(crate) fn try_save_active_workspace_file(&mut self) -> bool {
        let Some(request) = self.file_editor_host.save_request() else {
            return false;
        };
        self.write_active_workspace_file(request)
    }

    pub(crate) fn overwrite_active_workspace_file(&mut self) -> bool {
        let Some(request) = self.file_editor_host.overwrite_request() else {
            return false;
        };
        self.write_active_workspace_file(request)
    }

    fn write_active_workspace_file(&mut self, request: TextFileSaveRequest) -> bool {
        let path = request.path().to_owned();
        let Some(session) = self.agent_session.as_ref() else {
            return false;
        };
        let saved = match session.write_file(request) {
            Ok(version) => self.file_editor_host.mark_active_saved(version),
            Err(error) => {
                eprintln!("could not save App Server workspace file: {error}");
                if let Ok(snapshot) = session.read_file(path.clone()) {
                    self.file_editor_host.observe_external(snapshot);
                }
                false
            }
        };
        if saved {
            self.language_service.save(&path);
        }
        self.rebuild_presentation();
        self.request_redraw();
        saved
    }

    fn refresh_open_files_from_app_server(&mut self, changed: &FsChanged) {
        let paths = match changed {
            FsChanged::PathsChanged { paths, .. } => self
                .file_editor_host
                .tabs()
                .iter()
                .filter(|tab| paths.iter().any(|path| path == tab.path()))
                .map(|tab| tab.path().to_path_buf())
                .collect::<Vec<_>>(),
            FsChanged::RescanRequired { .. } => self
                .file_editor_host
                .tabs()
                .iter()
                .map(|tab| tab.path().to_path_buf())
                .collect(),
        };
        let Some(session) = self.agent_session.as_ref() else {
            return;
        };
        for path in paths {
            match session.read_file(path) {
                Ok(snapshot) => {
                    self.file_editor_host.observe_external(snapshot);
                }
                Err(error) => eprintln!("could not refresh open workspace file: {error}"),
            }
        }
    }
}

fn definition_editor_position(
    text: &str,
    row: u32,
    character: u32,
    encoding: zeta_language_service::LanguagePositionEncoding,
) -> Option<zeta_editor::CodeEditorPosition> {
    let row_index = usize::try_from(row).ok()?;
    let line = text.split('\n').nth(row_index)?;
    let line = line.strip_suffix('\r').unwrap_or(line);
    let requested = usize::try_from(character).ok()?;
    let byte_offset = match encoding {
        zeta_language_service::LanguagePositionEncoding::Utf8 => {
            (requested <= line.len() && line.is_char_boundary(requested)).then_some(requested)?
        }
        zeta_language_service::LanguagePositionEncoding::Utf16 => {
            let mut units = 0;
            let mut resolved = None;
            for (offset, scalar) in line.char_indices() {
                if units == requested {
                    resolved = Some(offset);
                    break;
                }
                units += scalar.len_utf16();
                if units > requested {
                    return None;
                }
            }
            resolved.or_else(|| (units == requested).then_some(line.len()))?
        }
    };
    Some(zeta_editor::CodeEditorPosition {
        row_index,
        byte_offset,
    })
}
