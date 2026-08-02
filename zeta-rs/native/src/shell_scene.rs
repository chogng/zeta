use zeta_keybinding::{KeyboardShortcuts, paint_chord_hint};
use zeta_terminal::{GridSize, ScreenBuffer, TerminalColor, TerminalCore, TerminalMousePosition};
use zeta_ui::{
    Border, CaretVisibility, Color, CornerRadii, Element, FontFamily, FontWeight, PaintRect, Rect,
    Sash, SashOrientation, SashState, SashStyle, SceneCheckpoint, ScrollMetrics,
    ScrollbarPresentation, TextBlock, TextInputLayoutEngine, TextStyle, UiScene,
};

use crate::PRODUCT_DISPLAY_NAME;
use crate::agent_composer::ComposerMode;
use crate::agent_sidebar::AgentSidebarState;
use crate::agent_sidebar_layout::AgentSidebarLayout;
use crate::agent_sidebar_toolbar::AgentSidebarToolbar;
use crate::agent_sidebar_workspace::AgentSidebarView;
use crate::agent_sidebar_workspace::AgentSidebarWorkspace;
use crate::composer_editor::ComposerEditor;
use crate::composer_interaction::ComposerInteractionModel;
use crate::composer_interaction_pane::ComposerInteractionPaneState;
use crate::composer_panel::{
    ComposerPanelLayout, ComposerPanelView, draw_composer_panel, interaction_preferred_height,
};
use crate::editor_pane::EditorPane;
use crate::explorer_pane::ExplorerPane;
use crate::file_editor_host::FileEditorHost;
use crate::file_editor_pane::{FileEditorPane, FileEditorPrompt};
use crate::file_editor_search::FileEditorSearchState;
use crate::git_branch_context_menu::{GitBranchContextMenu, GitBranchContextMenuState};
use crate::keybindings::NativeKeybindings;
use crate::keyboard_shortcuts::{
    KeyboardShortcutsState, keyboard_shortcut_rows, keyboard_shortcuts_ids,
};
use crate::language_server_settings::{LanguageServerSettings, LanguageServerSettingsState};
use crate::session_context_menu::{SessionContextMenu, SessionContextMenuState};
use crate::session_search::SessionSearch;
use crate::session_sidebar::SessionSidebarState;
use crate::session_sidebar_toolbar::SessionSidebarToolbar;
use crate::session_tab_list::{SessionTab, SessionTabList};
use crate::shell_interaction::{
    ACTIVE_SESSION_TAB, AGENT_FILE_SEARCH_INPUT, AGENT_SIDEBAR, FILE_EDITOR_DOCUMENT, MAIN_SURFACE,
    SESSION_SEARCH_INPUT, SESSION_SIDEBAR, SESSION_SIDEBAR_RESIZE_HANDLE, TERMINAL_OUTPUT,
    THREAD_TIMELINE, WINDOW,
};
use crate::shell_style::ShellPalette;
use crate::terminal_blocks::{TerminalBlockLineKind, project_block_lines};
use crate::terminal_output_scroll_view::TerminalOutputScrollView;
use crate::terminal_selection::{TerminalSelectionRange, paint_terminal_selection};
use crate::terminal_workspace_layout::TerminalWorkspaceLayout;
use crate::thread_projection::ThreadProjection;
use crate::thread_timeline::ThreadTimeline;
use crate::titlebar::{TITLEBAR_HEIGHT, Titlebar};
use crate::workspace_context::WorkspaceContext;
use crate::workspace_path_picker::{WorkspacePathPicker, WorkspacePathPickerState};
use crate::workspace_surface::WorkspaceSurfaceKind;
use zeta_editor::CodeEditorStyle;
use zeta_ui_dispatch::{
    AccessibilityNode, AccessibilityRole, CursorFeedback, InteractionFrame,
    InteractionFrameCheckpoint, UiDispatch, UiNode,
};
use zeta_winit::WindowControlInsets;

const TERMINAL_CELL_WIDTH: f32 = 8.0;
const TERMINAL_LINE_HEIGHT: f32 = 18.0;
const TERMINAL_PADDING: f32 = 24.0;
const COMPOSER_HEIGHT: f32 = 44.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct LogicalViewport {
    pub width: f32,
    pub height: f32,
}

impl LogicalViewport {
    pub(crate) fn from_physical(width: u32, height: u32, scale_factor: f64) -> Self {
        let scale_factor = if scale_factor.is_finite() && scale_factor > 0.0 {
            scale_factor as f32
        } else {
            1.0
        };
        Self {
            width: width as f32 / scale_factor,
            height: height as f32 / scale_factor,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct ShellLayout {
    titlebar: Rect,
    session_sidebar: Option<Rect>,
    session_sidebar_sash_track: Option<Rect>,
    agent_sidebar: Option<Rect>,
    main: Rect,
    output: Rect,
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
        let body_split = session_sidebar.layout(body);
        let session_sidebar = body_split
            .pane_bounds(0)
            .filter(|bounds| !bounds.is_empty());
        let remaining = body_split
            .pane_bounds(1)
            .expect("Sessions split must retain its main pane");
        let session_sidebar_sash_track = body_split.sash(0).map(|sash| sash.track_bounds());
        let terminal_workspace = TerminalWorkspaceLayout::for_bounds(remaining, agent_sidebar);
        let main = terminal_workspace.active_pane_bounds();
        let agent_sidebar = terminal_workspace.agent_sidebar_bounds();
        let composer_panel = ComposerPanelLayout::for_main(
            main,
            preferred_composer_height.max(COMPOSER_HEIGHT),
            preferred_interaction_height,
        );
        let output = composer_panel.output();
        Some(Self {
            titlebar,
            session_sidebar,
            session_sidebar_sash_track,
            agent_sidebar,
            main,
            output,
            composer_panel_layout: composer_panel,
            composer_panel: composer_panel.panel(),
            composer_info_bar: composer_panel.info_bar(),
            composer_toolbar: composer_panel.toolbar(),
            composer: composer_panel.editor(),
        })
    }
}

pub(crate) struct ShellPresentation {
    pub(crate) scene: UiScene,
    pub(crate) interaction_frame: InteractionFrame,
    pub(crate) accessibility_nodes: Vec<AccessibilityNode>,
    pub(crate) ime_cursor_area: Option<Rect>,
    pub(crate) workspace_path_picker_scroll_metrics: Option<ScrollMetrics>,
    pub(crate) workspace_path_picker_item_viewport: Option<Rect>,
    base_checkpoint: Option<ShellBaseCheckpoint>,
}

#[derive(Clone, Debug, PartialEq)]
struct ShellBaseCheckpoint {
    scene: SceneCheckpoint,
    interaction: InteractionFrameCheckpoint,
    ime_cursor_area: Option<Rect>,
}

struct ShellOverlayPresentation {
    ime_cursor_area: Option<Rect>,
    workspace_path_picker_scroll_metrics: Option<ScrollMetrics>,
    workspace_path_picker_item_viewport: Option<Rect>,
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
    pub(crate) composer: &'a ComposerEditor,
    pub(crate) composer_interaction: &'a ComposerInteractionModel,
    pub(crate) composer_interaction_pane: &'a ComposerInteractionPaneState,
    pub(crate) composer_mode: ComposerMode,
    pub(crate) session_search: &'a SessionSearch,
    pub(crate) caret_visibility: CaretVisibility,
    pub(crate) dispatch: &'a UiDispatch,
    pub(crate) session_sidebar: SessionSidebarState,
    pub(crate) agent_sidebar: AgentSidebarState,
    pub(crate) agent_sidebar_workspace: &'a AgentSidebarWorkspace,
    pub(crate) session_context_menu: SessionContextMenuState,
    pub(crate) git_branch_context_menu: &'a GitBranchContextMenuState,
    pub(crate) workspace_path_picker: &'a WorkspacePathPickerState,
    pub(crate) keybindings: &'a NativeKeybindings,
    pub(crate) keyboard_shortcuts: &'a KeyboardShortcutsState,
    pub(crate) language_server_settings: &'a LanguageServerSettingsState,
    pub(crate) language_server_runtime_state:
        Option<&'a zeta_language_service::LanguageServerState>,
    pub(crate) keybinding_diagnostics: &'a [String],
    pub(crate) window_control_insets: WindowControlInsets,
    pub(crate) pointer_position: Option<zeta_ui::Point>,
}

#[derive(Clone, Copy)]
struct SessionSidebarView<'a> {
    title: &'a str,
    context: &'a WorkspaceContext,
    search: &'a SessionSearch,
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
    thread_projection: &'a ThreadProjection,
    thread_timeline_scroll_offset: usize,
    composer: ComposerPanelView<'a>,
    file_editor: FileEditorPresentationView<'a>,
}

pub(crate) fn build_shell_presentation(
    viewport: LogicalViewport,
    model: ShellPresentationModel<'_>,
    text_layout: &mut TextInputLayoutEngine,
) -> ShellPresentation {
    let palette = model.palette;
    let mut scene = UiScene::new(palette.background);
    let mut interaction_frame = InteractionFrame::default();
    interaction_frame.register(UiNode::new(
        WINDOW,
        Rect::from_xywh(0.0, 0.0, viewport.width, viewport.height),
        AccessibilityRole::Window,
        PRODUCT_DISPLAY_NAME,
    ));
    let Some(layout) = ShellLayout::for_viewport_with_composer_and_interaction_height(
        viewport,
        model.session_sidebar,
        model.agent_sidebar,
        model.composer.preferred_height(),
        interaction_preferred_height(model.composer_interaction.view()),
    ) else {
        draw_compact_scene(&mut scene, viewport, palette);
        return ShellPresentation {
            scene,
            accessibility_nodes: interaction_frame.accessibility_nodes(model.dispatch),
            interaction_frame,
            ime_cursor_area: None,
            workspace_path_picker_scroll_metrics: None,
            workspace_path_picker_item_viewport: None,
            base_checkpoint: None,
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
    let titlebar = Titlebar::new(
        layout.titlebar,
        palette,
        model.session_sidebar,
        model.agent_sidebar,
        model.window_control_insets,
        model.dispatch,
    );
    titlebar.register_interactions(&mut interaction_frame);
    scene.draw_component(&titlebar);
    let session_search_caret = if let Some(bounds) = layout.session_sidebar {
        draw_session_sidebar(
            &mut scene,
            &mut interaction_frame,
            bounds,
            SessionSidebarView {
                title,
                context: model.workspace_context,
                search: model.session_search,
                caret_visibility: model.caret_visibility,
                dispatch: model.dispatch,
            },
            text_layout,
            palette,
        )
    } else {
        None
    };
    let file_search_caret = if let Some(bounds) = layout.agent_sidebar {
        draw_agent_sidebar(
            &mut scene,
            &mut interaction_frame,
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
    } else {
        None
    };
    let composer_caret = draw_main(
        &mut scene,
        &mut interaction_frame,
        layout,
        MainPresentationView {
            terminal: TerminalView {
                core: model.terminal,
                scroll_offset: model.terminal_scroll_offset,
                scrollbar_presentation: model.terminal_scrollbar_presentation,
                selection: model.terminal_selection,
            },
            workspace_surface: model.workspace_surface,
            thread_projection: model.thread_projection,
            thread_timeline_scroll_offset: model.thread_timeline_scroll_offset,
            composer: ComposerPanelView {
                context: model.workspace_context,
                editor: model.composer,
                interaction: model.composer_interaction,
                interaction_pane: model.composer_interaction_pane,
                mode: model.composer_mode,
                caret_visibility: model.caret_visibility,
                dispatch: model.dispatch,
            },
            file_editor: FileEditorPresentationView {
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
        },
        palette,
        text_layout,
    );
    let ime_cursor_area = if model.dispatch.is_focused(AGENT_FILE_SEARCH_INPUT) {
        file_search_caret
    } else if model.dispatch.is_focused(SESSION_SEARCH_INPUT) {
        session_search_caret
    } else {
        composer_caret
    };
    if let Some(bounds) = layout.session_sidebar_sash_track {
        draw_session_sidebar_sash(
            &mut scene,
            &mut interaction_frame,
            bounds,
            model.session_sidebar,
            model.dispatch,
            palette,
        );
    }
    let base_checkpoint = ShellBaseCheckpoint {
        scene: scene.checkpoint(),
        interaction: interaction_frame.checkpoint(),
        ime_cursor_area,
    };
    let overlay = draw_shell_overlays(
        &mut scene,
        &mut interaction_frame,
        viewport,
        &model,
        text_layout,
        ime_cursor_area,
    );
    let accessibility_nodes = interaction_frame.accessibility_nodes(model.dispatch);
    ShellPresentation {
        scene,
        interaction_frame,
        accessibility_nodes,
        ime_cursor_area: overlay.ime_cursor_area,
        workspace_path_picker_scroll_metrics: overlay.workspace_path_picker_scroll_metrics,
        workspace_path_picker_item_viewport: overlay.workspace_path_picker_item_viewport,
        base_checkpoint: Some(base_checkpoint),
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
    presentation.scene.restore(&base.scene);
    presentation.interaction_frame.restore(base.interaction);
    let overlay = draw_shell_overlays(
        &mut presentation.scene,
        &mut presentation.interaction_frame,
        viewport,
        &model,
        text_layout,
        base.ime_cursor_area,
    );
    presentation.accessibility_nodes = presentation
        .interaction_frame
        .accessibility_nodes(model.dispatch);
    presentation.ime_cursor_area = overlay.ime_cursor_area;
    presentation.workspace_path_picker_scroll_metrics =
        overlay.workspace_path_picker_scroll_metrics;
    presentation.workspace_path_picker_item_viewport = overlay.workspace_path_picker_item_viewport;
    true
}

fn draw_shell_overlays(
    scene: &mut UiScene,
    interaction_frame: &mut InteractionFrame,
    viewport: LogicalViewport,
    model: &ShellPresentationModel<'_>,
    text_layout: &mut TextInputLayoutEngine,
    mut ime_cursor_area: Option<Rect>,
) -> ShellOverlayPresentation {
    let palette = model.palette;
    let mut workspace_path_picker_scroll_metrics = None;
    let mut workspace_path_picker_item_viewport = None;
    if let Some(context_menu) = SessionContextMenu::new(
        Rect::from_xywh(0.0, 0.0, viewport.width, viewport.height),
        model.session_context_menu,
        palette,
        model.dispatch,
    ) {
        context_menu.register_interactions(interaction_frame);
        scene.draw_component(&context_menu);
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
        path_picker.register_interactions(interaction_frame);
        scene.draw_component(&path_picker);
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
        branch_menu.register_interactions(interaction_frame);
        scene.draw_component(&branch_menu);
    }
    let viewport_bounds = Rect::from_xywh(0.0, 0.0, viewport.width, viewport.height);
    if let Some((keybinding, entered)) = model.keybindings.pending_keybinding() {
        paint_chord_hint(
            scene,
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
        shortcuts.register_interactions(interaction_frame);
        scene.draw_component(&shortcuts);
    }
    if let Some(settings) = LanguageServerSettings::new(
        viewport_bounds,
        model.language_server_settings,
        model.caret_visibility,
        palette,
        text_layout,
        model.dispatch,
    ) {
        let settings = if let Some(runtime_state) = model.language_server_runtime_state {
            settings.with_runtime_state(runtime_state)
        } else {
            settings
        };
        if model
            .dispatch
            .is_focused(crate::language_server_settings::LANGUAGE_SERVER_EXECUTABLE_INPUT)
        {
            ime_cursor_area = settings.executable_caret_bounds();
        }
        settings.register_interactions(interaction_frame);
        scene.draw_component(&settings);
    }
    ShellOverlayPresentation {
        ime_cursor_area,
        workspace_path_picker_scroll_metrics,
        workspace_path_picker_item_viewport,
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
    scene: &mut UiScene,
    interaction_frame: &mut InteractionFrame,
    bounds: Rect,
    view: AgentSidebarPresentationView<'_>,
    text_layout: &mut TextInputLayoutEngine,
    palette: ShellPalette,
) -> Option<Rect> {
    scene.draw_rect(
        PaintRect::new(bounds, palette.surface_raised).with_border(Border::new(
            zeta_ui::Edges::new(0.0, 0.0, 0.0, 1.0),
            palette.border,
        )),
    );
    interaction_frame.register(
        UiNode::new(
            AGENT_SIDEBAR,
            bounds,
            AccessibilityRole::Group,
            "Agent sidebar",
        )
        .with_parent(WINDOW),
    );
    let layout = AgentSidebarLayout::for_bounds(bounds);
    let toolbar = AgentSidebarToolbar::new(
        layout.toolbar(),
        view.workspace,
        view.context,
        view.caret_visibility,
        palette,
        text_layout,
        view.dispatch,
    );
    toolbar.register_interactions(interaction_frame);
    scene.draw_component(&toolbar);
    match view.workspace.active_view() {
        AgentSidebarView::Changes => {
            let editor = EditorPane::new(layout.content(), view.workspace.editor(), palette);
            editor.register_interactions(interaction_frame);
            scene.draw_component(&editor);
        }
        AgentSidebarView::Files => {
            let explorer = ExplorerPane::new(layout.content(), view.workspace, palette);
            explorer.register_interactions(interaction_frame);
            scene.draw_component(&explorer);
        }
    }
    view.dispatch
        .is_focused(AGENT_FILE_SEARCH_INPUT)
        .then_some(toolbar.search_caret_bounds())
        .flatten()
}

fn draw_session_sidebar(
    scene: &mut UiScene,
    interaction_frame: &mut InteractionFrame,
    bounds: Rect,
    view: SessionSidebarView<'_>,
    text_layout: &mut TextInputLayoutEngine,
    palette: ShellPalette,
) -> Option<Rect> {
    scene.with_element(
        Element::leaf("SessionSidebar").in_bounds(bounds),
        |scene, _| {
            scene.draw_rect(PaintRect::new(bounds, palette.surface_raised).with_border(
                Border::new(zeta_ui::Edges::new(0.0, 1.0, 0.0, 0.0), palette.border),
            ));
            interaction_frame.register(
                UiNode::new(
                    SESSION_SIDEBAR,
                    bounds,
                    AccessibilityRole::Group,
                    "Sessions sidebar",
                )
                .with_parent(WINDOW),
            );
            let toolbar = SessionSidebarToolbar::new(
                bounds,
                view.search.input(),
                view.caret_visibility,
                palette,
                text_layout,
                view.dispatch,
            );
            toolbar.register_interactions(interaction_frame);
            scene.draw_component(&toolbar);
            let tabs = view
                .search
                .matches_session_name(view.title)
                .then(|| {
                    SessionTab::new(
                        ACTIVE_SESSION_TAB,
                        view.title,
                        view.context.working_directory_label(),
                        "Active",
                    )
                })
                .into_iter()
                .collect::<Vec<_>>();
            let tab_list = SessionTabList::new(
                SessionSidebarToolbar::content_bounds(bounds),
                &tabs,
                ACTIVE_SESSION_TAB,
                palette,
                view.dispatch,
            );
            tab_list.register_interactions(interaction_frame);
            scene.draw_component(&tab_list);
            view.dispatch
                .is_focused(SESSION_SEARCH_INPUT)
                .then_some(toolbar.search_caret_bounds())
                .flatten()
        },
    )
}

fn draw_session_sidebar_sash(
    scene: &mut UiScene,
    interaction_frame: &mut InteractionFrame,
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
    interaction_frame.register(
        UiNode::new(
            SESSION_SIDEBAR_RESIZE_HANDLE,
            sash.interaction_bounds(),
            AccessibilityRole::Separator,
            "Resize sessions sidebar",
        )
        .with_parent(WINDOW)
        .with_cursor(CursorFeedback::ResizeHorizontal)
        .with_value(format!("{} pixels", bounds.origin.x.round())),
    );
    scene.draw_component(&sash);
}

fn draw_main(
    scene: &mut UiScene,
    interaction_frame: &mut InteractionFrame,
    layout: ShellLayout,
    view: MainPresentationView<'_>,
    palette: ShellPalette,
    text_layout: &mut TextInputLayoutEngine,
) -> Option<Rect> {
    let active_screen = match view.workspace_surface {
        WorkspaceSurfaceKind::Terminal => ScreenBuffer::Alternate,
        WorkspaceSurfaceKind::Agent | WorkspaceSurfaceKind::Editor => ScreenBuffer::Primary,
    };
    scene.draw_rect(PaintRect::new(layout.main, palette.background));
    interaction_frame.register(
        UiNode::new(
            MAIN_SURFACE,
            layout.main,
            AccessibilityRole::Group,
            match view.workspace_surface {
                WorkspaceSurfaceKind::Agent => "Agent workspace",
                WorkspaceSurfaceKind::Editor => "File editor workspace",
                WorkspaceSurfaceKind::Terminal => "Interactive terminal",
            },
        )
        .with_parent(WINDOW)
        .with_cursor(CursorFeedback::Text),
    );
    match view.workspace_surface {
        WorkspaceSurfaceKind::Terminal => {
            interaction_frame.register(
                UiNode::new(
                    TERMINAL_OUTPUT,
                    terminal_content_bounds(layout, active_screen),
                    AccessibilityRole::Terminal,
                    "Interactive terminal",
                )
                .with_parent(MAIN_SURFACE)
                .with_cursor(CursorFeedback::Text),
            );
        }
        WorkspaceSurfaceKind::Agent => {
            interaction_frame.register(
                UiNode::new(
                    THREAD_TIMELINE,
                    layout.output,
                    AccessibilityRole::Group,
                    "Agent Thread timeline",
                )
                .with_parent(MAIN_SURFACE)
                .with_cursor(CursorFeedback::Text),
            );
        }
        WorkspaceSurfaceKind::Editor => {}
    }
    let mut ime_cursor_area = None;
    scene.with_clip(layout.main, |scene| match view.workspace_surface {
        WorkspaceSurfaceKind::Terminal => {
            draw_terminal(scene, layout, view.terminal, active_screen, palette);
        }
        WorkspaceSurfaceKind::Agent => {
            scene.draw_component(&ThreadTimeline::new(
                layout.output,
                view.thread_projection,
                view.thread_timeline_scroll_offset,
                palette,
            ));
            ime_cursor_area = draw_composer_panel(
                scene,
                interaction_frame,
                layout.composer_panel_layout,
                view.composer,
                text_layout,
                palette,
            );
        }
        WorkspaceSurfaceKind::Editor => {
            let caret_visibility = if view.file_editor.dispatch.is_focused(FILE_EDITOR_DOCUMENT) {
                view.file_editor.caret_visibility
            } else {
                CaretVisibility::Hidden
            };
            let pane = FileEditorPane::new(
                layout.main,
                view.file_editor.host,
                view.file_editor.style.clone(),
                palette,
                caret_visibility,
            )
            .with_prompt(view.file_editor.prompt)
            .with_diagnostics(view.file_editor.diagnostics)
            .with_language_features(
                view.file_editor.language_hover,
                view.file_editor.language_completions,
            )
            .with_completion_selection(view.file_editor.completion_selection)
            .with_pointer_position(view.file_editor.pointer_position)
            .with_search(
                view.file_editor.search,
                text_layout,
                view.file_editor.dispatch,
                view.file_editor.caret_visibility,
            );
            pane.register_interactions(interaction_frame);
            ime_cursor_area = if view.file_editor.dispatch.is_focused(FILE_EDITOR_DOCUMENT) {
                pane.caret_bounds()
            } else {
                view.file_editor
                    .dispatch
                    .focused()
                    .and_then(|focused| pane.search_caret_bounds(focused))
            };
            scene.draw_component(&pane);
        }
    });
    match view.workspace_surface {
        WorkspaceSurfaceKind::Terminal => view.terminal.core.and_then(|terminal| {
            terminal_cursor_area(layout, terminal, view.terminal.scroll_offset)
        }),
        WorkspaceSurfaceKind::Agent | WorkspaceSurfaceKind::Editor => ime_cursor_area,
    }
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
