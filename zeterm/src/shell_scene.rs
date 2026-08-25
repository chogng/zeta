use std::collections::BTreeMap;

use zeta_terminal::{GridSize, ScreenBuffer, TerminalColor, TerminalCore, TerminalMousePosition};
use zeta_ui::{
    Border, CaretVisibility, Color, CornerRadii, FontFamily, FontWeight, InteractionRegion,
    PaintRect, Rect, Sash, SashOrientation, SashState, SashStyle, SceneCheckpoint, ScrollMetrics,
    ScrollbarPresentation, SplitViewResizeSnapshot, TextBlock, TextInputLayoutEngine, TextStyle,
    UiScene,
};
use zeterm_keybinding_ui::KeyboardShortcuts;
use zeterm_keybinding_ui::paint_chord_hint;

use crate::PRODUCT_DISPLAY_NAME;
use crate::agent_sidebar::AgentSidebarState;
use crate::agent_sidebar_workspace::AgentSidebarView;
use crate::agent_sidebar_workspace::AgentSidebarWorkspace;
use crate::composer_panel::{ComposerPanelView, draw_composer_panel};
use crate::file_editor_host::FileEditorHost;
use crate::file_editor_pane::{FileEditorPane, FileEditorPrompt};
use crate::file_editor_search::FileEditorSearchState;
use crate::git_branch_context_menu::{GitBranchContextMenu, GitBranchContextMenuState};
use crate::keybindings::NativeKeybindings;
use crate::keyboard_shortcuts::{
    KeyboardShortcutsState, keyboard_shortcut_rows, keyboard_shortcuts_ids,
};
use crate::language_server_settings::{
    LanguageServerSettings, LanguageServerSettingsState, paint_switch_fragment,
};
use crate::remote_connection_manager::RemoteConnectionManagerField;
use crate::remote_connection_manager::RemoteConnectionManagerState;
use crate::remote_connection_manager_view::RemoteConnectionManager;
use crate::remote_connection_picker::RemoteConnectionPicker;
use crate::remote_connection_picker::RemoteConnectionPickerState;
use crate::remote_tunnel_manager::REMOTE_TUNNEL_REMOTE_PORT;
use crate::remote_tunnel_manager::RemoteTunnelManagerState;
use crate::remote_tunnel_manager_view::RemoteTunnelManager;
use crate::session_canvas::SessionCanvasLayout;
use crate::session_canvas::SessionHeader;
use crate::session_context_menu::{SessionContextMenu, SessionContextMenuState};
use crate::session_search::SessionSearch;
use crate::session_sidebar::SessionSidebarState;
use crate::session_sidebar_toolbar::SessionSidebarToolbar;
use crate::session_tab_list::{SessionTabState, WorkbenchTab, WorkbenchTabList};
use crate::settings_sections::SettingsSectionPane;
use crate::shell_interaction::{
    ACTIVE_SESSION_TAB, AGENT_FILE_SEARCH_INPUT, AGENT_SIDEBAR, AGENT_SIDEBAR_RESIZE_HANDLE,
    AGENT_SIDEBAR_TOOLBAR, FILE_EDITOR_DOCUMENT, FILE_EDITOR_FIND_INPUT, FILE_EDITOR_REPLACE_INPUT,
    MAIN_SURFACE, SESSION_SEARCH_INPUT, SESSION_SIDEBAR, SESSION_SIDEBAR_RESIZE_HANDLE,
    SETTINGS_WORKBENCH_TAB, TERMINAL_OUTPUT, THREAD_TIMELINE, WINDOW,
};
use crate::shell_style::ShellPalette;
use crate::terminal_blocks::{TerminalBlockLineKind, project_block_lines};
use crate::terminal_output_scroll_view::TerminalOutputScrollView;
use crate::terminal_selection::{TerminalSelectionRange, paint_terminal_selection};
use crate::thread_projection::ThreadProjection;
use crate::thread_timeline::ThreadTimeline;
use crate::titlebar::{TITLEBAR_HEIGHT, Titlebar};
use crate::workbench::WorkbenchItem;
use crate::workspace_context::WorkspaceContext;
use crate::workspace_path_picker::{WorkspacePathPicker, WorkspacePathPickerState};
use crate::workspace_surface::WorkspaceSurfaceKind;
use zeta_agent_sidebar::AgentSidebarNavigation;
use zeta_agent_sidebar::EditorPane;
use zeta_agent_sidebar::FilesLayout;
use zeta_agent_sidebar::FilesPane;
use zeta_agent_sidebar::FilesToolbar;
use zeta_agent_sidebar::ScmLayout;
use zeta_composer::Composer;
use zeta_composer::ComposerPanelLayout;
use zeta_editor::CodeEditorStyle;
use zeta_settings::SettingsPage;
use zeta_settings::SettingsPageActionAvailability;
use zeta_settings::SettingsPageMode;
use zeta_settings::SettingsPageSection;
use zeta_ui::layout::TerminalWorkspaceLayout;
use zui::ui::{
    AccessibilityNode, AccessibilityRole, ComponentContext, CursorFeedback, ElementId,
    InteractionFrame, InteractionFrameCheckpoint, UiDispatch, UiFrame,
};
use zui::window::WindowControlInsets;

const TERMINAL_CELL_WIDTH: f32 = 8.0;
const TERMINAL_LINE_HEIGHT: f32 = 18.0;
const TERMINAL_PADDING: f32 = 24.0;
const COMPOSER_HEIGHT: f32 = 44.0;

pub(crate) use zeta_ui::layout::LogicalViewport;

#[derive(Clone, Copy, Debug, PartialEq)]
struct ShellLayout {
    titlebar: Rect,
    session_sidebar: Option<Rect>,
    session_sidebar_sash_track: Option<Rect>,
    agent_sidebar: Option<Rect>,
    agent_sidebar_sash_track: Option<Rect>,
    agent_sidebar_resize_snapshot: Option<SplitViewResizeSnapshot>,
    main: Rect,
    output: Rect,
    session_header: Rect,
    thread_timeline: Rect,
    composer_panel_layout: ComposerPanelLayout,
    composer_panel: Rect,
    composer_info_bar: Rect,
    composer_toolbar: Rect,
    composer: Rect,
}

impl ShellLayout {
    fn for_viewport(
        viewport: LogicalViewport,
        session_sidebar: SessionSidebarState,
        agent_sidebar: AgentSidebarState,
    ) -> Option<Self> {
        Self::for_viewport_with_composer_and_interaction_height(
            viewport,
            session_sidebar,
            agent_sidebar,
            COMPOSER_HEIGHT,
            0.0,
        )
    }

    #[cfg(test)]
    fn for_viewport_with_composer_height(
        viewport: LogicalViewport,
        session_sidebar: SessionSidebarState,
        agent_sidebar: AgentSidebarState,
        preferred_composer_height: f32,
    ) -> Option<Self> {
        Self::for_viewport_with_composer_and_interaction_height(
            viewport,
            session_sidebar,
            agent_sidebar,
            preferred_composer_height,
            0.0,
        )
    }

    fn for_viewport_with_composer_and_interaction_height(
        viewport: LogicalViewport,
        session_sidebar: SessionSidebarState,
        agent_sidebar: AgentSidebarState,
        preferred_composer_height: f32,
        preferred_interaction_height: f32,
    ) -> Option<Self> {
        if viewport.width < 240.0 || viewport.height < 180.0 {
            return None;
        }
        let titlebar = Rect::from_xywh(0.0, 0.0, viewport.width, TITLEBAR_HEIGHT);
        let body_height = viewport.height - titlebar.size.height;
        let body = Rect::from_xywh(0.0, titlebar.bottom(), viewport.width, body_height);
        let sessions_layout = session_sidebar.layout(body);
        let session_sidebar = sessions_layout.sessions_bounds();
        let remaining = sessions_layout.main_bounds();
        let session_sidebar_sash_track = sessions_layout.sash_track();
        let terminal_workspace =
            TerminalWorkspaceLayout::for_bounds(remaining, agent_sidebar.layout_spec());
        let main = terminal_workspace.active_pane_bounds();
        let agent_sidebar = terminal_workspace.sidebar_bounds();
        let agent_sidebar_sash_track = terminal_workspace.sidebar_sash_track();
        let agent_sidebar_resize_snapshot = terminal_workspace.sidebar_resize_snapshot();
        let composer_panel = ComposerPanelLayout::for_main(
            main,
            preferred_composer_height.max(COMPOSER_HEIGHT),
            preferred_interaction_height,
        );
        let output = composer_panel.output();
        let session_canvas = SessionCanvasLayout::for_output(output);
        Some(Self {
            titlebar,
            session_sidebar,
            session_sidebar_sash_track,
            agent_sidebar,
            agent_sidebar_sash_track,
            agent_sidebar_resize_snapshot,
            main,
            output,
            session_header: session_canvas.header(),
            thread_timeline: session_canvas.timeline(),
            composer_panel_layout: composer_panel,
            composer_panel: composer_panel.panel(),
            composer_info_bar: composer_panel.info_bar(),
            composer_toolbar: composer_panel.toolbar(),
            composer: composer_panel.editor(),
        })
    }
}

pub(crate) fn agent_sidebar_resize_snapshot_for_viewport(
    viewport: LogicalViewport,
    session_sidebar: SessionSidebarState,
    agent_sidebar: AgentSidebarState,
) -> Option<SplitViewResizeSnapshot> {
    ShellLayout::for_viewport(viewport, session_sidebar, agent_sidebar)
        .and_then(|layout| layout.agent_sidebar_resize_snapshot)
}

pub(crate) struct ShellPresentation {
    pub(crate) frame: UiFrame<InteractionFrame>,
    pub(crate) accessibility_nodes: Vec<AccessibilityNode>,
    pub(crate) ime_cursor_area: Option<Rect>,
    pub(crate) workspace_path_picker_scroll_metrics: Option<ScrollMetrics>,
    pub(crate) workspace_path_picker_item_viewport: Option<Rect>,
    pub(crate) remote_connection_picker_scroll_metrics: Option<ScrollMetrics>,
    pub(crate) remote_connection_picker_item_viewport: Option<Rect>,
    pub(crate) remote_connection_manager_scroll_metrics: Option<ScrollMetrics>,
    pub(crate) remote_connection_manager_list_viewport: Option<Rect>,
    pub(crate) remote_tunnel_manager_scroll_metrics: Option<ScrollMetrics>,
    pub(crate) remote_tunnel_manager_list_viewport: Option<Rect>,
    pub(crate) language_server_settings_content: Option<Rect>,
    base_checkpoint: Option<ShellBaseCheckpoint>,
    retained_fragments: BTreeMap<ElementId, RetainedFragmentCheckpoint>,
}

impl ShellPresentation {
    pub(crate) fn scene(&self) -> &UiScene {
        self.frame.scene()
    }

    pub(crate) fn scene_mut(&mut self) -> &mut UiScene {
        self.frame.scene_mut()
    }

    pub(crate) fn interaction_frame(&self) -> &InteractionFrame {
        self.frame.interaction()
    }

    pub(crate) fn interaction_frame_mut(&mut self) -> &mut InteractionFrame {
        self.frame.interaction_mut()
    }

    /// Removes a retained shell fragment and restores both shared frame outputs to its mount
    /// checkpoint. A failed terminal-fragment removal tells the host to rebuild the presentation.
    pub(crate) fn remove_retained_fragment(
        &mut self,
        id: ElementId,
    ) -> Result<(), zeta_ui::SceneFragmentError> {
        let Some(checkpoint) = self.retained_fragments.get(&id).cloned() else {
            return Err(zeta_ui::SceneFragmentError::Missing(id));
        };
        self.scene_mut().remove_fragment(id)?;
        self.interaction_frame_mut().restore(checkpoint.interaction);
        self.retained_fragments.remove(&id);
        Ok(())
    }

    pub(crate) fn record_retained_fragment(&mut self, id: ElementId) {
        self.retained_fragments.insert(
            id,
            RetainedFragmentCheckpoint {
                scene: self.scene().checkpoint(),
                interaction: self.interaction_frame().checkpoint(),
            },
        );
    }

    pub(crate) fn forget_retained_fragment(&mut self, id: ElementId) {
        self.retained_fragments.remove(&id);
    }
}

#[derive(Clone, Debug, PartialEq)]
struct ShellBaseCheckpoint {
    scene: SceneCheckpoint,
    interaction: InteractionFrameCheckpoint,
    ime_cursor_area: Option<Rect>,
}

#[derive(Clone, Debug, PartialEq)]
struct RetainedFragmentCheckpoint {
    scene: SceneCheckpoint,
    interaction: InteractionFrameCheckpoint,
}

struct ShellOverlayPresentation {
    ime_cursor_area: Option<Rect>,
    workspace_path_picker_scroll_metrics: Option<ScrollMetrics>,
    workspace_path_picker_item_viewport: Option<Rect>,
    remote_connection_picker_scroll_metrics: Option<ScrollMetrics>,
    remote_connection_picker_item_viewport: Option<Rect>,
    remote_connection_manager_scroll_metrics: Option<ScrollMetrics>,
    remote_connection_manager_list_viewport: Option<Rect>,
    remote_tunnel_manager_scroll_metrics: Option<ScrollMetrics>,
    remote_tunnel_manager_list_viewport: Option<Rect>,
}

#[derive(Clone, Copy)]
struct TerminalView<'a> {
    core: Option<&'a TerminalCore>,
    scroll_offset: usize,
    scrollbar_presentation: ScrollbarPresentation,
    selection: Option<TerminalSelectionRange>,
}

#[derive(Clone, Copy)]
pub(crate) struct ShellPresentationModel<'a> {
    pub(crate) palette: ShellPalette,
    pub(crate) terminal: Option<&'a TerminalCore>,
    pub(crate) terminal_scroll_offset: usize,
    pub(crate) terminal_scrollbar_presentation: ScrollbarPresentation,
    pub(crate) terminal_selection: Option<TerminalSelectionRange>,
    pub(crate) workspace_surface: WorkspaceSurfaceKind,
    pub(crate) file_editor_host: &'a FileEditorHost,
    pub(crate) file_editor_prompt: FileEditorPrompt,
    pub(crate) file_editor_search: &'a FileEditorSearchState,
    pub(crate) file_editor_diagnostics: &'a [zeta_editor::CodeEditorDiagnostic],
    pub(crate) language_hover: Option<&'a zeta_language_service::LanguageHover>,
    pub(crate) language_completions: Option<&'a zeta_language_service::LanguageCompletions>,
    pub(crate) completion_selection: usize,
    pub(crate) code_editor_style: &'a CodeEditorStyle,
    pub(crate) thread_projection: &'a ThreadProjection,
    pub(crate) thread_timeline_scroll_offset: usize,
    pub(crate) workspace_context: &'a WorkspaceContext,
    pub(crate) composer: &'a Composer,
    pub(crate) session_search: &'a SessionSearch,
    pub(crate) session_tabs: &'a [SessionTabState],
    pub(crate) selected_session_tab: ElementId,
    pub(crate) workbench_item: WorkbenchItem,
    pub(crate) caret_visibility: CaretVisibility,
    pub(crate) dispatch: &'a UiDispatch,
    pub(crate) session_sidebar: SessionSidebarState,
    pub(crate) agent_sidebar: AgentSidebarState,
    pub(crate) agent_sidebar_workspace: &'a AgentSidebarWorkspace,
    pub(crate) session_context_menu: SessionContextMenuState,
    pub(crate) git_branch_context_menu: &'a GitBranchContextMenuState,
    pub(crate) workspace_path_picker: &'a WorkspacePathPickerState,
    pub(crate) remote_connection_picker: &'a RemoteConnectionPickerState,
    pub(crate) remote_connection_manager: &'a RemoteConnectionManagerState,
    pub(crate) remote_tunnel_manager: &'a RemoteTunnelManagerState,
    pub(crate) keybindings: &'a NativeKeybindings,
    pub(crate) keyboard_shortcuts: &'a KeyboardShortcutsState,
    pub(crate) language_server_settings: &'a LanguageServerSettingsState,
    pub(crate) settings_section: SettingsPageSection,
    pub(crate) language_server_runtime_state:
        Option<&'a zeta_language_service::LanguageServerState>,
    pub(crate) keybinding_diagnostics: &'a [String],
    pub(crate) theme_scheme: zeta_theme::ColorScheme,
    pub(crate) theme_follows_system: bool,
    pub(crate) window_control_insets: WindowControlInsets,
    pub(crate) pointer_position: Option<zeta_ui::Point>,
}

#[derive(Clone, Copy)]
struct SessionSidebarView<'a> {
    title: &'a str,
    context: &'a WorkspaceContext,
    search: &'a SessionSearch,
    tabs: &'a [SessionTabState],
    workbench_item: WorkbenchItem,
    caret_visibility: CaretVisibility,
    dispatch: &'a UiDispatch,
}

#[derive(Clone, Copy)]
struct AgentSidebarPresentationView<'a> {
    workspace: &'a AgentSidebarWorkspace,
    context: &'a WorkspaceContext,
    caret_visibility: CaretVisibility,
    dispatch: &'a UiDispatch,
}

#[derive(Clone, Copy)]
struct FileEditorPresentationView<'a> {
    host: &'a FileEditorHost,
    prompt: FileEditorPrompt,
    search: &'a FileEditorSearchState,
    diagnostics: &'a [zeta_editor::CodeEditorDiagnostic],
    language_hover: Option<&'a zeta_language_service::LanguageHover>,
    language_completions: Option<&'a zeta_language_service::LanguageCompletions>,
    completion_selection: usize,
    style: &'a CodeEditorStyle,
    caret_visibility: CaretVisibility,
    dispatch: &'a UiDispatch,
    pointer_position: Option<zeta_ui::Point>,
}

#[derive(Clone, Copy)]
struct MainPresentationView<'a> {
    terminal: TerminalView<'a>,
    workspace_surface: WorkspaceSurfaceKind,
    workbench_item: WorkbenchItem,
    language_server_settings: &'a LanguageServerSettingsState,
    settings_section: SettingsPageSection,
    language_server_runtime_state: Option<&'a zeta_language_service::LanguageServerState>,
    session_title: &'a str,
    thread_projection: &'a ThreadProjection,
    thread_timeline_scroll_offset: usize,
    composer: ComposerPanelView<'a>,
    workspace_context: &'a WorkspaceContext,
    keybindings: &'a NativeKeybindings,
    keyboard_shortcuts: &'a KeyboardShortcutsState,
    keybinding_diagnostics: &'a [String],
    theme_scheme: zeta_theme::ColorScheme,
    theme_follows_system: bool,
    caret_visibility: CaretVisibility,
    dispatch: &'a UiDispatch,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct MainDrawResult {
    ime_cursor_area: Option<Rect>,
    settings_content: Option<Rect>,
}

#[cfg(test)]
pub(crate) fn build_shell_presentation(
    viewport: LogicalViewport,
    model: ShellPresentationModel<'_>,
    text_layout: &mut TextInputLayoutEngine,
) -> ShellPresentation {
    build_shell_presentation_with_bindings(viewport, model, text_layout, None)
}

pub(crate) fn build_shell_presentation_with_animation_bindings(
    viewport: LogicalViewport,
    model: ShellPresentationModel<'_>,
    text_layout: &mut TextInputLayoutEngine,
    animation_bindings: &mut dyn zui::ui::AnimationBinding,
) -> ShellPresentation {
    build_shell_presentation_with_bindings(viewport, model, text_layout, Some(animation_bindings))
}

fn build_shell_presentation_with_bindings(
    viewport: LogicalViewport,
    model: ShellPresentationModel<'_>,
    text_layout: &mut TextInputLayoutEngine,
    mut animation_bindings: Option<&mut dyn zui::ui::AnimationBinding>,
) -> ShellPresentation {
    let palette = model.palette;
    let mut frame = UiFrame::<InteractionFrame>::new(palette.background);
    frame.draw_component(&InteractionRegion::new(
        "Window",
        WINDOW,
        Rect::from_xywh(0.0, 0.0, viewport.width, viewport.height),
        AccessibilityRole::Window,
        PRODUCT_DISPLAY_NAME,
    ));
    let Some(layout) = ShellLayout::for_viewport_with_composer_and_interaction_height(
        viewport,
        model.session_sidebar,
        model.agent_sidebar,
        model.composer.input().preferred_height(),
        model.composer.interaction().view().map_or(0.0, |view| {
            zeta_composer::interaction_preferred_height(view.items().len())
        }),
    ) else {
        draw_compact_scene(frame.scene_mut(), viewport, palette);
        return ShellPresentation {
            accessibility_nodes: frame.interaction().accessibility_nodes(model.dispatch),
            frame,
            ime_cursor_area: None,
            workspace_path_picker_scroll_metrics: None,
            workspace_path_picker_item_viewport: None,
            remote_connection_picker_scroll_metrics: None,
            remote_connection_picker_item_viewport: None,
            remote_connection_manager_scroll_metrics: None,
            remote_connection_manager_list_viewport: None,
            remote_tunnel_manager_scroll_metrics: None,
            remote_tunnel_manager_list_viewport: None,
            language_server_settings_content: None,
            base_checkpoint: None,
            retained_fragments: BTreeMap::new(),
        };
    };

    let title = if model.workspace_surface == WorkspaceSurfaceKind::Terminal {
        model
            .terminal
            .and_then(TerminalCore::title)
            .unwrap_or("Terminal")
    } else {
        model
            .thread_projection
            .thread()
            .map(|thread| thread.title.as_str())
            .unwrap_or(PRODUCT_DISPLAY_NAME)
    };
    let session_title = model
        .session_tabs
        .iter()
        .find(|tab| tab.id() == model.selected_session_tab)
        .map(SessionTabState::title)
        .unwrap_or("New session");
    let titlebar = Titlebar::new(
        layout.titlebar,
        palette,
        model.session_sidebar,
        model.agent_sidebar,
        model.window_control_insets,
        model.dispatch,
    );
    frame.draw_component(&titlebar);
    let session_search_caret = if let Some(bounds) = layout.session_sidebar {
        frame.with_context(|context| {
            draw_session_sidebar(
                context,
                bounds,
                SessionSidebarView {
                    title,
                    context: model.workspace_context,
                    search: model.session_search,
                    tabs: model.session_tabs,
                    workbench_item: model.workbench_item,
                    caret_visibility: model.caret_visibility,
                    dispatch: model.dispatch,
                },
                text_layout,
                palette,
            )
        })
    } else {
        None
    };
    let mut file_editor_caret = None;
    let file_search_caret = if let Some(bounds) = layout.agent_sidebar {
        if model.workspace_surface == WorkspaceSurfaceKind::Editor {
            file_editor_caret = frame.with_context(|context| {
                draw_file_editor_inspector(
                    context,
                    bounds,
                    FileEditorPresentationView {
                        host: model.file_editor_host,
                        prompt: model.file_editor_prompt,
                        search: model.file_editor_search,
                        diagnostics: model.file_editor_diagnostics,
                        language_hover: model.language_hover,
                        language_completions: model.language_completions,
                        completion_selection: model.completion_selection,
                        style: model.code_editor_style,
                        caret_visibility: model.caret_visibility,
                        dispatch: model.dispatch,
                        pointer_position: model.pointer_position,
                    },
                    text_layout,
                    palette,
                )
            });
            None
        } else {
            match animation_bindings.as_deref_mut() {
                Some(animation_bindings) => {
                    frame.with_animation_bindings(animation_bindings, |context| {
                        draw_agent_sidebar(
                            context,
                            bounds,
                            AgentSidebarPresentationView {
                                workspace: model.agent_sidebar_workspace,
                                context: model.workspace_context,
                                caret_visibility: model.caret_visibility,
                                dispatch: model.dispatch,
                            },
                            text_layout,
                            palette,
                        )
                    })
                }
                None => frame.with_context(|context| {
                    draw_agent_sidebar(
                        context,
                        bounds,
                        AgentSidebarPresentationView {
                            workspace: model.agent_sidebar_workspace,
                            context: model.workspace_context,
                            caret_visibility: model.caret_visibility,
                            dispatch: model.dispatch,
                        },
                        text_layout,
                        palette,
                    )
                }),
            }
        }
    } else {
        None
    };
    let main_draw = |context: &mut ComponentContext<'_, '_>| {
        draw_main(
            context,
            layout,
            MainPresentationView {
                terminal: TerminalView {
                    core: model.terminal,
                    scroll_offset: model.terminal_scroll_offset,
                    scrollbar_presentation: model.terminal_scrollbar_presentation,
                    selection: model.terminal_selection,
                },
                workspace_surface: model.workspace_surface,
                workbench_item: model.workbench_item,
                language_server_settings: model.language_server_settings,
                settings_section: model.settings_section,
                language_server_runtime_state: model.language_server_runtime_state,
                session_title,
                thread_projection: model.thread_projection,
                thread_timeline_scroll_offset: model.thread_timeline_scroll_offset,
                composer: ComposerPanelView {
                    context: model.workspace_context,
                    editor: model.composer.input(),
                    interaction: model.composer.interaction(),
                    interaction_pane: model.composer.interaction_pane(),
                    route: model.composer.route(),
                    caret_visibility: model.caret_visibility,
                    dispatch: model.dispatch,
                },
                workspace_context: model.workspace_context,
                keybindings: model.keybindings,
                keyboard_shortcuts: model.keyboard_shortcuts,
                keybinding_diagnostics: model.keybinding_diagnostics,
                theme_scheme: model.theme_scheme,
                theme_follows_system: model.theme_follows_system,
                caret_visibility: model.caret_visibility,
                dispatch: model.dispatch,
            },
            palette,
            text_layout,
        )
    };
    let main_draw = match animation_bindings.as_deref_mut() {
        Some(animation_bindings) => frame.with_animation_bindings(animation_bindings, main_draw),
        None => frame.with_context(main_draw),
    };
    if let Some(bounds) = layout.agent_sidebar {
        draw_agent_sidebar_border(frame.scene_mut(), bounds, palette);
    }
    if let Some(bounds) = layout.agent_sidebar_sash_track {
        frame.with_context(|context| {
            draw_agent_sidebar_sash(
                context,
                bounds,
                model.agent_sidebar,
                model.dispatch,
                palette,
            )
        });
    }
    let ime_cursor_area = if model.dispatch.is_focused(FILE_EDITOR_DOCUMENT)
        || model.dispatch.is_focused(FILE_EDITOR_FIND_INPUT)
        || model.dispatch.is_focused(FILE_EDITOR_REPLACE_INPUT)
    {
        file_editor_caret
    } else if model.dispatch.is_focused(AGENT_FILE_SEARCH_INPUT) {
        file_search_caret
    } else if model.dispatch.is_focused(SESSION_SEARCH_INPUT) {
        session_search_caret
    } else {
        main_draw.ime_cursor_area
    };
    if let Some(bounds) = layout.session_sidebar_sash_track {
        frame.with_context(|context| {
            draw_session_sidebar_sash(
                context,
                bounds,
                model.session_sidebar,
                model.dispatch,
                palette,
            )
        });
    }
    let base_checkpoint = ShellBaseCheckpoint {
        scene: frame.scene().checkpoint(),
        interaction: frame.interaction().checkpoint(),
        ime_cursor_area,
    };
    let overlay = draw_shell_overlays(&mut frame, viewport, &model, text_layout, ime_cursor_area);
    let accessibility_nodes = frame.interaction().accessibility_nodes(model.dispatch);
    ShellPresentation {
        frame,
        accessibility_nodes,
        ime_cursor_area: overlay.ime_cursor_area,
        workspace_path_picker_scroll_metrics: overlay.workspace_path_picker_scroll_metrics,
        workspace_path_picker_item_viewport: overlay.workspace_path_picker_item_viewport,
        remote_connection_picker_scroll_metrics: overlay.remote_connection_picker_scroll_metrics,
        remote_connection_picker_item_viewport: overlay.remote_connection_picker_item_viewport,
        remote_connection_manager_scroll_metrics: overlay.remote_connection_manager_scroll_metrics,
        remote_connection_manager_list_viewport: overlay.remote_connection_manager_list_viewport,
        remote_tunnel_manager_scroll_metrics: overlay.remote_tunnel_manager_scroll_metrics,
        remote_tunnel_manager_list_viewport: overlay.remote_tunnel_manager_list_viewport,
        language_server_settings_content: main_draw.settings_content,
        base_checkpoint: Some(base_checkpoint),
        retained_fragments: BTreeMap::new(),
    }
}

/// Replaces only volatile shell overlays while retaining base layout, paint, and interaction data.
pub(crate) fn rebuild_shell_overlays(
    presentation: &mut ShellPresentation,
    viewport: LogicalViewport,
    model: ShellPresentationModel<'_>,
    text_layout: &mut TextInputLayoutEngine,
) -> bool {
    let Some(base) = presentation.base_checkpoint.clone() else {
        return false;
    };
    presentation.scene_mut().restore(&base.scene);
    presentation
        .interaction_frame_mut()
        .restore(base.interaction);
    let overlay = draw_shell_overlays(
        &mut presentation.frame,
        viewport,
        &model,
        text_layout,
        base.ime_cursor_area,
    );
    presentation.accessibility_nodes = presentation
        .interaction_frame()
        .accessibility_nodes(model.dispatch);
    presentation.ime_cursor_area = overlay.ime_cursor_area;
    presentation.workspace_path_picker_scroll_metrics =
        overlay.workspace_path_picker_scroll_metrics;
    presentation.workspace_path_picker_item_viewport = overlay.workspace_path_picker_item_viewport;
    presentation.remote_connection_picker_scroll_metrics =
        overlay.remote_connection_picker_scroll_metrics;
    presentation.remote_connection_picker_item_viewport =
        overlay.remote_connection_picker_item_viewport;
    presentation.remote_connection_manager_scroll_metrics =
        overlay.remote_connection_manager_scroll_metrics;
    presentation.remote_connection_manager_list_viewport =
        overlay.remote_connection_manager_list_viewport;
    presentation.remote_tunnel_manager_scroll_metrics =
        overlay.remote_tunnel_manager_scroll_metrics;
    presentation.remote_tunnel_manager_list_viewport = overlay.remote_tunnel_manager_list_viewport;
    true
}

/// Replaces one product-owned retained scene fragment by its stable interaction ID.
pub(crate) fn rebuild_shell_fragment(
    presentation: &mut ShellPresentation,
    id: ElementId,
    state: &LanguageServerSettingsState,
    palette: ShellPalette,
    dispatch: &UiDispatch,
    progress: f32,
) -> bool {
    let Some(panel) = presentation.language_server_settings_content else {
        return false;
    };
    if id != crate::language_server_settings::LANGUAGE_SERVER_SWITCH {
        return false;
    }
    presentation
        .scene_mut()
        .replace_fragment(id, |scene| {
            paint_switch_fragment(scene, panel, state, palette, dispatch, progress);
        })
        .is_ok()
}

fn draw_shell_overlays(
    frame: &mut UiFrame<InteractionFrame>,
    viewport: LogicalViewport,
    model: &ShellPresentationModel<'_>,
    text_layout: &mut TextInputLayoutEngine,
    mut ime_cursor_area: Option<Rect>,
) -> ShellOverlayPresentation {
    let palette = model.palette;
    let mut workspace_path_picker_scroll_metrics = None;
    let mut workspace_path_picker_item_viewport = None;
    let mut remote_connection_picker_scroll_metrics = None;
    let mut remote_connection_picker_item_viewport = None;
    let mut remote_connection_manager_scroll_metrics = None;
    let mut remote_connection_manager_list_viewport = None;
    let mut remote_tunnel_manager_scroll_metrics = None;
    let mut remote_tunnel_manager_list_viewport = None;
    if let Some(context_menu) = SessionContextMenu::new(
        Rect::from_xywh(0.0, 0.0, viewport.width, viewport.height),
        model.session_context_menu,
        palette,
        model.dispatch,
    ) {
        frame.draw_component(&context_menu);
    }
    if let Some(path_picker) = WorkspacePathPicker::new(
        Rect::from_xywh(0.0, 0.0, viewport.width, viewport.height),
        model.workspace_path_picker,
        model.caret_visibility,
        palette,
        text_layout,
        model.dispatch,
    ) {
        workspace_path_picker_scroll_metrics = path_picker.scroll_metrics();
        workspace_path_picker_item_viewport = Some(path_picker.item_viewport_bounds());
        if model
            .dispatch
            .is_focused(crate::workspace_path_picker::WORKSPACE_PATH_SEARCH_INPUT)
        {
            ime_cursor_area = path_picker.search_caret_bounds();
        }
        frame.draw_component(&path_picker);
    }
    if let Some(connection_picker) = RemoteConnectionPicker::new(
        Rect::from_xywh(0.0, 0.0, viewport.width, viewport.height),
        model.remote_connection_picker,
        model.caret_visibility,
        palette,
        text_layout,
        model.dispatch,
    ) {
        remote_connection_picker_scroll_metrics = connection_picker.scroll_metrics();
        remote_connection_picker_item_viewport = Some(connection_picker.item_viewport_bounds());
        if model
            .dispatch
            .is_focused(crate::remote_connection_picker::REMOTE_CONNECTION_SEARCH_INPUT)
        {
            ime_cursor_area = connection_picker.search_caret_bounds();
        }
        frame.draw_component(&connection_picker);
    }
    if let Some(connection_manager) = RemoteConnectionManager::new(
        Rect::from_xywh(0.0, 0.0, viewport.width, viewport.height),
        model.remote_connection_manager,
        model.caret_visibility,
        palette,
        text_layout,
        model.dispatch,
    ) {
        remote_connection_manager_scroll_metrics = Some(connection_manager.list_scroll_metrics());
        remote_connection_manager_list_viewport = Some(connection_manager.list_viewport_bounds());
        for field in [
            RemoteConnectionManagerField::Name,
            RemoteConnectionManagerField::Host,
            RemoteConnectionManagerField::Workspace,
        ] {
            if model.dispatch.is_focused(field.element_id()) {
                ime_cursor_area = connection_manager.caret_bounds(field);
            }
        }
        frame.draw_component(&connection_manager);
    }
    if let Some(tunnel_manager) = RemoteTunnelManager::new(
        Rect::from_xywh(0.0, 0.0, viewport.width, viewport.height),
        model.remote_tunnel_manager,
        model.caret_visibility,
        palette,
        text_layout,
        model.dispatch,
    ) {
        remote_tunnel_manager_scroll_metrics = Some(tunnel_manager.list_scroll_metrics());
        remote_tunnel_manager_list_viewport = Some(tunnel_manager.list_viewport_bounds());
        if model.dispatch.is_focused(REMOTE_TUNNEL_REMOTE_PORT) {
            ime_cursor_area = tunnel_manager.remote_port_caret_bounds();
        }
        frame.draw_component(&tunnel_manager);
    }
    if let Some(branch_menu) = GitBranchContextMenu::new(
        Rect::from_xywh(0.0, 0.0, viewport.width, viewport.height),
        model.git_branch_context_menu,
        model.caret_visibility,
        palette,
        text_layout,
        model.dispatch,
    ) {
        if model
            .dispatch
            .is_focused(crate::git_branch_context_menu::GIT_BRANCH_SEARCH_INPUT)
        {
            ime_cursor_area = branch_menu.search_caret_bounds();
        }
        frame.draw_component(&branch_menu);
    }
    let viewport_bounds = Rect::from_xywh(0.0, 0.0, viewport.width, viewport.height);
    if let Some((keybinding, entered)) = model.keybindings.pending_keybinding() {
        paint_chord_hint(
            frame.scene_mut(),
            viewport_bounds,
            keybinding,
            entered,
            model.keybindings.platform(),
        );
    }
    let shortcut_rows = keyboard_shortcut_rows(model.keybindings);
    if let Some(shortcuts) = KeyboardShortcuts::new(
        viewport_bounds,
        model.keyboard_shortcuts,
        &shortcut_rows,
        model.keybinding_diagnostics,
        keyboard_shortcuts_ids(),
        model.keybindings.platform(),
        model.dispatch,
    ) {
        frame.draw_component(&shortcuts);
    }
    ShellOverlayPresentation {
        ime_cursor_area,
        workspace_path_picker_scroll_metrics,
        workspace_path_picker_item_viewport,
        remote_connection_picker_scroll_metrics,
        remote_connection_picker_item_viewport,
        remote_connection_manager_scroll_metrics,
        remote_connection_manager_list_viewport,
        remote_tunnel_manager_scroll_metrics,
        remote_tunnel_manager_list_viewport,
    }
}

pub(crate) fn terminal_grid_size_for_viewport(
    viewport: LogicalViewport,
    active_screen: ScreenBuffer,
    session_sidebar: SessionSidebarState,
    agent_sidebar: AgentSidebarState,
) -> GridSize {
    let Some(layout) = ShellLayout::for_viewport(viewport, session_sidebar, agent_sidebar) else {
        return GridSize::default();
    };
    let bounds = terminal_content_bounds(layout, active_screen);
    GridSize::new(
        (bounds.size.height / TERMINAL_LINE_HEIGHT)
            .floor()
            .clamp(1.0, u16::MAX as f32) as u16,
        (bounds.size.width / TERMINAL_CELL_WIDTH)
            .floor()
            .clamp(1.0, u16::MAX as f32) as u16,
    )
}

pub(crate) fn terminal_mouse_position_for_viewport(
    viewport: LogicalViewport,
    active_screen: ScreenBuffer,
    session_sidebar: SessionSidebarState,
    agent_sidebar: AgentSidebarState,
    point: zeta_ui::Point,
) -> Option<TerminalMousePosition> {
    let layout = ShellLayout::for_viewport(viewport, session_sidebar, agent_sidebar)?;
    let bounds = terminal_content_bounds(layout, active_screen);
    if !bounds.contains(point) {
        return None;
    }
    let row = ((point.y - bounds.origin.y) / TERMINAL_LINE_HEIGHT).floor() as u16;
    let col = ((point.x - bounds.origin.x) / TERMINAL_CELL_WIDTH).floor() as u16;
    let size =
        terminal_grid_size_for_viewport(viewport, active_screen, session_sidebar, agent_sidebar);
    (row < size.rows() && col < size.cols()).then(|| TerminalMousePosition::new(row, col))
}

fn draw_agent_sidebar(
    context: &mut ComponentContext<'_, '_>,
    bounds: Rect,
    view: AgentSidebarPresentationView<'_>,
    text_layout: &mut TextInputLayoutEngine,
    palette: ShellPalette,
) -> Option<Rect> {
    let sidebar = InteractionRegion::new(
        "AgentSidebar",
        AGENT_SIDEBAR,
        bounds,
        AccessibilityRole::Group,
        "Workspace inspector",
    )
    .with_parent(WINDOW);
    let active_view = view.workspace.active_view();
    let toolbar_bounds = match active_view {
        AgentSidebarView::Changes => ScmLayout::for_bounds(bounds).toolbar(),
        AgentSidebarView::Files => FilesLayout::for_bounds(bounds).toolbar(),
    };
    let content_bounds = match active_view {
        AgentSidebarView::Changes => ScmLayout::for_bounds(bounds).content(),
        AgentSidebarView::Files => FilesLayout::for_bounds(bounds).content(),
    };
    context.with_component(&sidebar, |context, _| {
        draw_agent_sidebar_surface(context.scene_mut(), bounds, palette);
        let toolbar = InteractionRegion::new(
            "AgentSidebarToolbar",
            AGENT_SIDEBAR_TOOLBAR,
            toolbar_bounds,
            AccessibilityRole::Toolbar,
            "Workspace inspector toolbar",
        )
        .with_parent(AGENT_SIDEBAR);
        let sidebar_style = palette.agent_sidebar_style();
        let search_caret = context.with_component(&toolbar, |context, _| {
            context.scene_mut().draw_rect(
                PaintRect::new(toolbar_bounds, palette.surface_raised).with_border(Border::new(
                    zeta_ui::Edges::new(0.0, 0.0, 1.0, 0.0),
                    palette.border,
                )),
            );
            let navigation = AgentSidebarNavigation::new(
                AgentSidebarNavigation::bounds_in(toolbar_bounds),
                active_view,
                &sidebar_style,
                view.dispatch,
            );
            context.draw_component(&navigation);
            match active_view {
                AgentSidebarView::Changes => None,
                AgentSidebarView::Files => {
                    let files_toolbar = FilesToolbar::new(
                        toolbar_bounds,
                        AgentSidebarNavigation::bounds_in(toolbar_bounds),
                        view.workspace.files(),
                        view.context.upstream_distance(),
                        view.caret_visibility,
                        sidebar_style,
                        text_layout,
                        view.dispatch,
                    );
                    let search_caret = files_toolbar.search_caret_bounds();
                    context.draw_component(&files_toolbar);
                    Some(search_caret)
                }
            }
        });
        match active_view {
            AgentSidebarView::Changes => {
                let editor = EditorPane::new(
                    content_bounds,
                    view.workspace.editor(),
                    palette.scm_pane_style(),
                );
                context.draw_component(&editor);
            }
            AgentSidebarView::Files => {
                let files_style = palette.files_pane_style();
                let explorer = FilesPane::new(
                    content_bounds,
                    view.workspace.files(),
                    AGENT_SIDEBAR,
                    &files_style,
                    view.dispatch,
                );
                context.draw_component(&explorer);
            }
        }
        view.dispatch
            .is_focused(AGENT_FILE_SEARCH_INPUT)
            .then_some(search_caret.flatten())
            .flatten()
    })
}

fn draw_file_editor_inspector(
    context: &mut ComponentContext<'_, '_>,
    bounds: Rect,
    view: FileEditorPresentationView<'_>,
    text_layout: &mut TextInputLayoutEngine,
    palette: ShellPalette,
) -> Option<Rect> {
    let inspector = InteractionRegion::new(
        "InspectorWorkbench",
        AGENT_SIDEBAR,
        bounds,
        AccessibilityRole::Group,
        "Inspector workbench · File editor",
    )
    .with_parent(WINDOW);
    context.with_component(&inspector, |context, _| {
        draw_agent_sidebar_surface(context.scene_mut(), bounds, palette);
        let pane = FileEditorPane::new(
            bounds,
            view.host,
            view.style.clone(),
            palette,
            if view.dispatch.is_focused(FILE_EDITOR_DOCUMENT) {
                view.caret_visibility
            } else {
                CaretVisibility::Hidden
            },
        )
        .with_parent(AGENT_SIDEBAR)
        .with_prompt(view.prompt)
        .with_diagnostics(view.diagnostics)
        .with_language_features(view.language_hover, view.language_completions)
        .with_completion_selection(view.completion_selection)
        .with_pointer_position(view.pointer_position)
        .with_search(
            view.search,
            text_layout,
            view.dispatch,
            view.caret_visibility,
        );
        let ime_cursor_area = if view.dispatch.is_focused(FILE_EDITOR_DOCUMENT) {
            pane.caret_bounds()
        } else {
            view.dispatch
                .focused()
                .and_then(|focused| pane.search_caret_bounds(focused))
        };
        context.draw_component(&pane);
        ime_cursor_area
    })
}

/// Paints the Native-owned surface of the Agent Sidebar before feature content.
fn draw_agent_sidebar_surface(scene: &mut UiScene, bounds: Rect, palette: ShellPalette) {
    scene.draw_rect(PaintRect::new(bounds, palette.surface_raised));
}

/// Paints the Native-owned outer border of the Agent Sidebar after feature content.
///
/// This boundary separates the right sidebar shell slot from the main
/// workspace. Files and SCM components own only their internal geometry and
/// must not redraw this edge.
fn draw_agent_sidebar_border(scene: &mut UiScene, bounds: Rect, palette: ShellPalette) {
    scene.draw_rect(
        PaintRect::new(bounds, Color::TRANSPARENT).with_border(Border::new(
            zeta_ui::Edges::new(0.0, 0.0, 0.0, 1.0),
            palette.border,
        )),
    );
}

fn draw_session_sidebar(
    context: &mut ComponentContext<'_, '_>,
    bounds: Rect,
    view: SessionSidebarView<'_>,
    text_layout: &mut TextInputLayoutEngine,
    palette: ShellPalette,
) -> Option<Rect> {
    let sidebar = InteractionRegion::new(
        "SessionSidebar",
        SESSION_SIDEBAR,
        bounds,
        AccessibilityRole::Group,
        "Workbench navigator",
    )
    .with_parent(WINDOW);
    context.with_component(&sidebar, |context, _| {
        context
            .scene_mut()
            .draw_rect(
                PaintRect::new(bounds, palette.surface_raised).with_border(Border::new(
                    zeta_ui::Edges::new(0.0, 1.0, 0.0, 0.0),
                    palette.border,
                )),
            );
        let toolbar = SessionSidebarToolbar::new(
            bounds,
            view.search.input(),
            view.caret_visibility,
            palette,
            text_layout,
            view.dispatch,
        );
        let search_caret = toolbar.search_caret_bounds();
        context.draw_component(&toolbar);
        let mut tabs = if view.tabs.is_empty() {
            view.search
                .matches_session_name(view.title)
                .then(|| {
                    WorkbenchTab::new(
                        ACTIVE_SESSION_TAB,
                        view.title,
                        view.context.working_directory_label(),
                        "Active",
                    )
                })
                .into_iter()
                .collect::<Vec<_>>()
        } else {
            view.tabs
                .iter()
                .filter(|tab| view.search.matches_session_name(tab.title()))
                .map(|tab| {
                    WorkbenchTab::new(tab.id(), tab.title(), tab.workspace(), tab.status_label())
                })
                .collect::<Vec<_>>()
        };
        tabs.push(WorkbenchTab::settings(SETTINGS_WORKBENCH_TAB));
        let tab_list = WorkbenchTabList::new(
            SessionSidebarToolbar::content_bounds(bounds),
            &tabs,
            view.workbench_item.element_id(),
            palette,
            view.dispatch,
        );
        context.draw_component(&tab_list);
        view.dispatch
            .is_focused(SESSION_SEARCH_INPUT)
            .then_some(search_caret)
            .flatten()
    })
}

fn draw_session_sidebar_sash(
    context: &mut ComponentContext<'_, '_>,
    bounds: Rect,
    session_sidebar: SessionSidebarState,
    dispatch: &UiDispatch,
    palette: ShellPalette,
) {
    let state = if session_sidebar.is_resizing() {
        SashState::Active
    } else if dispatch.is_hovered(SESSION_SIDEBAR_RESIZE_HANDLE) {
        SashState::Hovered
    } else {
        SashState::Resting
    };
    let sash = Sash::new(
        bounds,
        SashOrientation::Vertical,
        state,
        SashStyle::new(palette.accent),
    );
    context.draw_component(
        &InteractionRegion::new(
            "SessionSidebarResizeHandle",
            SESSION_SIDEBAR_RESIZE_HANDLE,
            sash.interaction_bounds(),
            AccessibilityRole::Separator,
            "Resize sessions sidebar",
        )
        .with_parent(WINDOW)
        .with_cursor(CursorFeedback::ResizeHorizontal)
        .with_value(format!("{} pixels", bounds.origin.x.round())),
    );
    context.draw_component(&sash);
}

fn draw_agent_sidebar_sash(
    context: &mut ComponentContext<'_, '_>,
    bounds: Rect,
    agent_sidebar: AgentSidebarState,
    dispatch: &UiDispatch,
    palette: ShellPalette,
) {
    let state = if agent_sidebar.is_resizing() {
        SashState::Active
    } else if dispatch.is_hovered(AGENT_SIDEBAR_RESIZE_HANDLE) {
        SashState::Hovered
    } else {
        SashState::Resting
    };
    let sash = Sash::new(
        bounds,
        SashOrientation::Vertical,
        state,
        SashStyle::new(palette.accent),
    );
    context.draw_component(
        &InteractionRegion::new(
            "AgentSidebarResizeHandle",
            AGENT_SIDEBAR_RESIZE_HANDLE,
            sash.interaction_bounds(),
            AccessibilityRole::Separator,
            "Resize inspector",
        )
        .with_parent(WINDOW)
        .with_cursor(CursorFeedback::ResizeHorizontal)
        .with_value(format!("{} pixels", bounds.origin.x.round())),
    );
    context.draw_component(&sash);
}

fn draw_main(
    context: &mut ComponentContext<'_, '_>,
    layout: ShellLayout,
    view: MainPresentationView<'_>,
    palette: ShellPalette,
    text_layout: &mut TextInputLayoutEngine,
) -> MainDrawResult {
    let active_screen = match view.workspace_surface {
        WorkspaceSurfaceKind::Terminal => ScreenBuffer::Alternate,
        WorkspaceSurfaceKind::Agent | WorkspaceSurfaceKind::Editor => ScreenBuffer::Primary,
    };
    let main_label = if view.workbench_item.is_settings() {
        "Settings"
    } else {
        match view.workspace_surface {
            WorkspaceSurfaceKind::Agent => "Agent workspace",
            WorkspaceSurfaceKind::Editor => "Agent session with file inspector",
            WorkspaceSurfaceKind::Terminal => "Interactive terminal",
        }
    };
    let main_surface = InteractionRegion::new(
        "MainSurface",
        MAIN_SURFACE,
        layout.main,
        AccessibilityRole::Group,
        main_label,
    )
    .with_parent(WINDOW)
    .with_cursor(CursorFeedback::Text);
    context.with_component(&main_surface, |context, _| {
        context
            .scene_mut()
            .draw_rect(PaintRect::new(layout.main, palette.background));
        context.with_clip(layout.main, |context| {
            let mut ime_cursor_area = None;
            let mut settings_content = None;
            if view.workbench_item.is_settings() {
                let actions = if view.settings_section == SettingsPageSection::LanguageServers {
                    SettingsPageActionAvailability::none()
                        .with_reset_enabled(view.language_server_settings.can_reset())
                        .with_save_enabled(view.language_server_settings.can_save())
                } else {
                    SettingsPageActionAvailability::none()
                };
                let settings_page = SettingsPage::new_with_header_height_and_section(
                    layout.main,
                    TITLEBAR_HEIGHT,
                    view.language_server_settings.search_input(),
                    view.caret_visibility,
                    palette.settings_page_style(),
                    actions,
                    view.settings_section,
                    view.dispatch,
                    text_layout,
                )
                .with_parent(MAIN_SURFACE)
                .with_mode(SettingsPageMode::Surface);
                if view.settings_section == SettingsPageSection::LanguageServers {
                    settings_content = Some(settings_page.content_bounds());
                }
                if view
                    .dispatch
                    .is_focused(zeta_settings::SETTINGS_SEARCH_INPUT)
                {
                    ime_cursor_area = settings_page.search_caret_bounds();
                }
                context.draw_component(&settings_page);
                match view.settings_section {
                    SettingsPageSection::LanguageServers => {
                        if let Some(settings) = LanguageServerSettings::new_in_content(
                            settings_page.content_bounds(),
                            view.language_server_settings,
                            view.caret_visibility,
                            palette,
                            text_layout,
                            view.dispatch,
                        ) {
                            let settings =
                                if let Some(runtime_state) = view.language_server_runtime_state {
                                    settings.with_runtime_state(runtime_state)
                                } else {
                                    settings
                                }
                                .without_switch_fragment()
                                .with_parent(zeta_settings::SETTINGS_PAGE);
                            if view.dispatch.is_focused(
                                crate::language_server_settings::LANGUAGE_SERVER_EXECUTABLE_INPUT,
                            ) {
                                ime_cursor_area = settings.executable_caret_bounds();
                            }
                            context.draw_component(&settings);
                        }
                    }
                    section => context.draw_component(&SettingsSectionPane::new(
                        settings_page.content_bounds(),
                        section,
                        palette,
                        view.workspace_context,
                        view.workspace_surface,
                        view.keybindings,
                        view.keyboard_shortcuts,
                        view.keybinding_diagnostics,
                        view.theme_scheme,
                        view.theme_follows_system,
                        view.dispatch,
                    )),
                }
            } else {
                match view.workspace_surface {
                    WorkspaceSurfaceKind::Terminal => {
                        let terminal_region = InteractionRegion::new(
                            "TerminalOutput",
                            TERMINAL_OUTPUT,
                            terminal_content_bounds(layout, active_screen),
                            AccessibilityRole::Terminal,
                            "Interactive terminal",
                        )
                        .with_parent(MAIN_SURFACE)
                        .with_cursor(CursorFeedback::Text);
                        context.with_component(&terminal_region, |context, _| {
                            draw_terminal(
                                context.scene_mut(),
                                layout,
                                view.terminal,
                                active_screen,
                                palette,
                            );
                        });
                    }
                    WorkspaceSurfaceKind::Agent | WorkspaceSurfaceKind::Editor => {
                        context.draw_component(&SessionHeader::new(
                            layout.session_header,
                            view.session_title,
                            view.thread_projection,
                            view.composer.context,
                            palette,
                        ));
                        let timeline_region = InteractionRegion::new(
                            "ThreadTimeline",
                            THREAD_TIMELINE,
                            layout.thread_timeline,
                            AccessibilityRole::Group,
                            "Agent Thread timeline",
                        )
                        .with_parent(MAIN_SURFACE)
                        .with_cursor(CursorFeedback::Text);
                        context.with_component(&timeline_region, |context, _| {
                            context.draw_component(&ThreadTimeline::new(
                                layout.thread_timeline,
                                view.thread_projection,
                                view.thread_timeline_scroll_offset,
                                palette,
                            ));
                        });
                        ime_cursor_area = draw_composer_panel(
                            context,
                            layout.composer_panel_layout,
                            view.composer,
                            text_layout,
                            palette,
                        );
                    }
                }
            }
            let ime_cursor_area = if view.workbench_item.is_settings() {
                ime_cursor_area
            } else {
                match view.workspace_surface {
                    WorkspaceSurfaceKind::Terminal => view.terminal.core.and_then(|terminal| {
                        terminal_cursor_area(layout, terminal, view.terminal.scroll_offset)
                    }),
                    WorkspaceSurfaceKind::Agent | WorkspaceSurfaceKind::Editor => ime_cursor_area,
                }
            };
            MainDrawResult {
                ime_cursor_area,
                settings_content,
            }
        })
    })
}

fn terminal_content_bounds(layout: ShellLayout, active_screen: ScreenBuffer) -> Rect {
    let viewport = if active_screen == ScreenBuffer::Alternate {
        layout.main
    } else {
        layout.output
    };
    Rect::from_xywh(
        viewport.origin.x + TERMINAL_PADDING,
        viewport.origin.y + TERMINAL_PADDING,
        (viewport.size.width - TERMINAL_PADDING * 2.0).max(1.0),
        (viewport.size.height - TERMINAL_PADDING * 2.0).max(1.0),
    )
}

fn terminal_cursor_area(
    layout: ShellLayout,
    terminal: &TerminalCore,
    scroll_offset: usize,
) -> Option<Rect> {
    if scroll_offset != 0 || !terminal.modes().cursor_visible() {
        return None;
    }
    let bounds = terminal_content_bounds(layout, ScreenBuffer::Alternate);
    let (row, col) = terminal.grid().cursor();
    Some(Rect::from_xywh(
        bounds.origin.x + col as f32 * TERMINAL_CELL_WIDTH,
        bounds.origin.y + row as f32 * TERMINAL_LINE_HEIGHT,
        TERMINAL_CELL_WIDTH,
        TERMINAL_LINE_HEIGHT,
    ))
}

fn draw_terminal(
    scene: &mut UiScene,
    layout: ShellLayout,
    view: TerminalView<'_>,
    active_screen: ScreenBuffer,
    palette: ShellPalette,
) {
    let bounds = terminal_content_bounds(layout, active_screen);
    let Some(terminal) = view.core else {
        draw_terminal_text(scene, "Starting shell…", bounds, palette.text_muted);
        return;
    };
    if active_screen == ScreenBuffer::Alternate {
        draw_grid(scene, terminal, bounds, view.scroll_offset, palette);
    } else {
        draw_block_list(
            scene,
            terminal,
            bounds,
            view.scroll_offset,
            view.scrollbar_presentation,
            view.selection,
            palette,
        );
    }
}

fn draw_grid(
    scene: &mut UiScene,
    terminal: &TerminalCore,
    bounds: Rect,
    scroll_offset: usize,
    palette: ShellPalette,
) {
    let cursor = terminal.grid().cursor();
    let cursor_visible = terminal.modes().cursor_visible() && scroll_offset == 0;
    for (row, line) in terminal.grid().viewport_lines(scroll_offset).enumerate() {
        let y = bounds.origin.y + row as f32 * TERMINAL_LINE_HEIGHT;
        if y + TERMINAL_LINE_HEIGHT > bounds.bottom() {
            break;
        }
        for (col, cell) in line.cells().iter().enumerate() {
            if cell.is_continuation() {
                continue;
            }
            let x = bounds.origin.x + col as f32 * TERMINAL_CELL_WIDTH;
            if x + TERMINAL_CELL_WIDTH > bounds.right() {
                break;
            }
            let style = cell.style();
            let (foreground, background) = terminal_cell_colors(style, palette);
            if background != Color::TRANSPARENT {
                scene.draw_rect(PaintRect::new(
                    Rect::from_xywh(x, y, TERMINAL_CELL_WIDTH, TERMINAL_LINE_HEIGHT),
                    background,
                ));
            }
            if cursor_visible && cursor == (row, col) {
                scene.draw_rect(PaintRect::new(
                    Rect::from_xywh(x, y + TERMINAL_LINE_HEIGHT - 2.0, TERMINAL_CELL_WIDTH, 2.0),
                    palette.accent,
                ));
            }
            if !cell.text().is_empty() {
                let mut text_style = terminal_text_style(foreground);
                if style.bold {
                    text_style = text_style.with_weight(FontWeight::Bold);
                }
                draw_text(
                    scene,
                    cell.text(),
                    Rect::from_xywh(x, y, TERMINAL_CELL_WIDTH * 2.0, TERMINAL_LINE_HEIGHT),
                    text_style,
                );
            }
        }
    }
}

fn draw_block_list(
    scene: &mut UiScene,
    terminal: &TerminalCore,
    bounds: Rect,
    scroll_offset: usize,
    scrollbar_presentation: ScrollbarPresentation,
    selection: Option<TerminalSelectionRange>,
    palette: ShellPalette,
) {
    let lines = project_block_lines(terminal);
    TerminalOutputScrollView::new(
        bounds,
        lines.len(),
        TERMINAL_LINE_HEIGHT,
        scroll_offset,
        scrollbar_presentation,
        palette,
    )
    .draw(scene, |scene, viewport, range| {
        for absolute_index in range {
            let line = &lines[absolute_index];
            let color = match line.kind {
                TerminalBlockLineKind::Preamble => palette.text_muted,
                TerminalBlockLineKind::Command => palette.accent,
                TerminalBlockLineKind::Output => palette.text,
                TerminalBlockLineKind::Status => palette.text_muted,
            };
            draw_terminal_text(
                scene,
                &line.text,
                Rect::from_xywh(
                    viewport.content_origin().x,
                    viewport.content_origin().y + absolute_index as f32 * TERMINAL_LINE_HEIGHT,
                    viewport.bounds().size.width,
                    TERMINAL_LINE_HEIGHT,
                ),
                color,
            );
        }
        if let Some(selection) = selection {
            paint_terminal_selection(
                scene,
                viewport.bounds(),
                terminal.grid().size().cols() as usize,
                selection,
                TERMINAL_CELL_WIDTH,
                TERMINAL_LINE_HEIGHT,
                palette.terminal_selection,
            );
        }
    });
}

fn terminal_cell_colors(style: zeta_terminal::CellStyle, palette: ShellPalette) -> (Color, Color) {
    let mut foreground = terminal_color(style.foreground, palette.text, palette);
    let mut background = terminal_color(style.background, Color::TRANSPARENT, palette);
    if style.inverse {
        std::mem::swap(&mut foreground, &mut background);
    }
    (foreground, background)
}

fn terminal_color(color: TerminalColor, default: Color, palette: ShellPalette) -> Color {
    match color {
        TerminalColor::Default => default,
        TerminalColor::Indexed(index) => palette.terminal_indexed_color(index),
        TerminalColor::Rgb(red, green, blue) => Color::rgb(red, green, blue),
    }
}

fn terminal_text_style(color: Color) -> TextStyle {
    TextStyle::new(13.0, color)
        .with_family(FontFamily::Monospace)
        .with_line_height(TERMINAL_LINE_HEIGHT)
}

fn draw_terminal_text(scene: &mut UiScene, text: &str, bounds: Rect, color: Color) {
    draw_text(scene, text, bounds, terminal_text_style(color));
}

fn draw_compact_scene(scene: &mut UiScene, viewport: LogicalViewport, palette: ShellPalette) {
    let bounds = Rect::from_xywh(
        12.0,
        12.0,
        (viewport.width - 24.0).max(1.0),
        (viewport.height - 24.0).max(1.0),
    );
    scene.draw_rect(
        PaintRect::new(bounds, palette.surface)
            .with_border(Border::uniform(1.0, palette.border))
            .with_corner_radii(CornerRadii::uniform(10.0)),
    );
    draw_text(
        scene,
        PRODUCT_DISPLAY_NAME,
        Rect::from_xywh(
            bounds.origin.x + 18.0,
            bounds.origin.y + 18.0,
            (bounds.size.width - 36.0).max(1.0),
            30.0,
        ),
        TextStyle::new(20.0, palette.text).with_weight(FontWeight::Bold),
    );
}

fn draw_text(scene: &mut UiScene, text: &str, bounds: Rect, style: TextStyle) {
    if bounds.is_empty() {
        return;
    }
    scene.draw_text(TextBlock::new(text, bounds.origin, bounds.size, style));
}

#[cfg(test)]
#[path = "shell_scene_tests.rs"]
mod tests;
