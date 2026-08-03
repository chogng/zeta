use super::{
    LogicalViewport, ShellLayout, ShellPresentation, ShellPresentationModel,
    build_shell_presentation, rebuild_shell_overlays, terminal_grid_size_for_viewport,
    terminal_mouse_position_for_viewport,
};
use crate::PRODUCT_DISPLAY_NAME;
use crate::agent_composer::ComposerMode;
use crate::agent_sidebar::AgentSidebarState;
use crate::agent_sidebar_workspace::{AgentSidebarView, AgentSidebarWorkspace};
use crate::composer_editor::ComposerEditor;
use crate::composer_interaction::ComposerInteractionModel;
use crate::composer_interaction_pane::ComposerInteractionPaneState;
use crate::file_editor_host::FileEditorHost;
use crate::git_branch_context_menu::GitBranchContextMenuState;
use crate::keybindings::NativeKeybindings;
use crate::keyboard_shortcuts::KeyboardShortcutsState;
use crate::language_server_settings::LanguageServerSettingsState;
use crate::session_context_menu::SessionContextMenuState;
use crate::session_search::SessionSearch;
use crate::session_sidebar::SessionSidebarState;
use crate::shell_interaction::{
    ACTIVE_SESSION_TAB, ADD_SESSION, AGENT_CHANGES, AGENT_EDITOR_PANE, AGENT_EXPLORER_PANE,
    AGENT_FILES, AGENT_FILES_REFRESH, AGENT_FILES_SEARCH, AGENT_SIDEBAR, AGENT_SIDEBAR_NAVIGATION,
    AGENT_SIDEBAR_RESIZE_HANDLE, AGENT_SIDEBAR_TOOLBAR, COMPOSER, COMPOSER_INFO_BAR, COMPOSER_MODE,
    COMPOSER_PANEL, ContextAction, FILE_EDITOR_DOCUMENT, FILE_EDITOR_PANE, FILE_EDITOR_TAB_LIST,
    MULTI_DIFF_EDITOR, SESSION_CONTEXT_MENU, SESSION_SEARCH_INPUT, SESSION_SIDEBAR_RESIZE_HANDLE,
    THREAD_TIMELINE, TITLEBAR,
};
use crate::thread_projection::ThreadProjection;
use crate::workspace_context::WorkspaceContext;
use crate::workspace_path_picker::WorkspacePathPickerState;
use crate::workspace_surface::WorkspaceSurfaceKind;
use zeta_editor::CodeEditorStyle;
use zeta_terminal::{GridSize, ScreenBuffer, TerminalCore};
use zeta_text_file::{TextFileAccess, TextFileDiskVersion, TextFileModifiedAt, TextFileSnapshot};
use zeta_ui::{
    CaretVisibility, Color, Edges, Point, Rect, ScrollbarPresentation, TextInputCommand,
    TextInputLayoutEngine, UiScene,
};
use zeta_winit::WindowControlInsets;
use zui::{AccessibilityRole, CursorFeedback, DispatchInvalidation, UiDispatch, UiIntent};

fn viewport() -> LogicalViewport {
    LogicalViewport {
        width: 1000.0,
        height: 700.0,
    }
}

#[test]
fn agent_sidebar_outer_border_is_owned_by_native_shell() {
    let bounds = Rect::from_xywh(680.0, 40.0, 320.0, 660.0);
    let mut scene = UiScene::new(crate::shell_style::SHELL_PALETTE.background);

    super::draw_agent_sidebar_border(&mut scene, bounds, crate::shell_style::SHELL_PALETTE);

    let frame = scene.rects().first().copied().expect("sidebar frame");
    assert_eq!(frame.bounds(), bounds);
    assert_eq!(frame.fill(), Color::TRANSPARENT);
    assert_eq!(frame.border().widths(), Edges::new(0.0, 0.0, 0.0, 1.0));
    assert_eq!(
        frame.border().color(),
        crate::shell_style::SHELL_PALETTE.border
    );
}

fn presentation(terminal: Option<&TerminalCore>, scroll_offset: usize) -> ShellPresentation {
    presentation_with_sidebar(terminal, scroll_offset, SessionSidebarState::default())
}

fn presentation_with_sidebar(
    terminal: Option<&TerminalCore>,
    scroll_offset: usize,
    session_sidebar: SessionSidebarState,
) -> ShellPresentation {
    presentation_with_sidebar_and_menu(
        terminal,
        scroll_offset,
        session_sidebar,
        SessionContextMenuState::default(),
    )
}

fn presentation_with_sidebar_and_menu(
    terminal: Option<&TerminalCore>,
    scroll_offset: usize,
    session_sidebar: SessionSidebarState,
    session_context_menu: SessionContextMenuState,
) -> ShellPresentation {
    presentation_with_sidebars_and_menu(
        terminal,
        scroll_offset,
        session_sidebar,
        AgentSidebarState::default(),
        session_context_menu,
    )
}

fn presentation_with_sidebars_and_menu(
    terminal: Option<&TerminalCore>,
    scroll_offset: usize,
    session_sidebar: SessionSidebarState,
    agent_sidebar: AgentSidebarState,
    session_context_menu: SessionContextMenuState,
) -> ShellPresentation {
    let composer = ComposerEditor::default();
    let composer_interaction = ComposerInteractionModel::new();
    let composer_interaction_pane = ComposerInteractionPaneState::default();
    let session_search = SessionSearch::default();
    let workspace_context = WorkspaceContext::fixture("~/Desktop/zeta", Some("main"), Some(0));
    let mut text_layout = TextInputLayoutEngine::new();
    let mut dispatch = UiDispatch::default();
    let agent_sidebar_workspace = AgentSidebarWorkspace::default();
    let file_editor_host = FileEditorHost::default();
    let code_editor_style = CodeEditorStyle::light();
    let thread_projection = ThreadProjection::default();
    let initial = build_shell_presentation(
        viewport(),
        ShellPresentationModel {
            palette: crate::shell_style::SHELL_PALETTE,
            terminal,
            terminal_scroll_offset: scroll_offset,
            terminal_scrollbar_presentation: ScrollbarPresentation::default(),
            terminal_selection: None,
            workspace_surface: if terminal
                .is_some_and(|terminal| terminal.active_screen() == ScreenBuffer::Alternate)
            {
                WorkspaceSurfaceKind::Terminal
            } else {
                WorkspaceSurfaceKind::Agent
            },
            file_editor_host: &file_editor_host,
            file_editor_prompt: crate::file_editor_pane::FileEditorPrompt::None,
            file_editor_search: &crate::file_editor_search::FileEditorSearchState::default(),
            file_editor_diagnostics: &[],
            language_hover: None,
            language_completions: None,
            completion_selection: 0,
            code_editor_style: &code_editor_style,
            thread_projection: &thread_projection,
            thread_timeline_scroll_offset: 0,
            workspace_context: &workspace_context,
            composer: &composer,
            composer_interaction: &composer_interaction,
            composer_interaction_pane: &composer_interaction_pane,
            composer_mode: ComposerMode::Agent,
            session_search: &session_search,
            caret_visibility: CaretVisibility::Visible,
            dispatch: &dispatch,
            session_sidebar,
            agent_sidebar,
            agent_sidebar_workspace: &agent_sidebar_workspace,
            session_context_menu,
            git_branch_context_menu: &GitBranchContextMenuState::default(),
            workspace_path_picker: &WorkspacePathPickerState::default(),
            keybindings: &NativeKeybindings::default(),
            keyboard_shortcuts: &KeyboardShortcutsState::default(),
            language_server_settings: &LanguageServerSettingsState::default(),
            language_server_runtime_state: None,
            keybinding_diagnostics: &[],
            window_control_insets: WindowControlInsets::NONE,
            pointer_position: None,
        },
        &mut text_layout,
    );
    dispatch.reconcile_focus(&initial.interaction_frame, COMPOSER);
    build_shell_presentation(
        viewport(),
        ShellPresentationModel {
            palette: crate::shell_style::SHELL_PALETTE,
            terminal,
            terminal_scroll_offset: scroll_offset,
            terminal_scrollbar_presentation: ScrollbarPresentation::default(),
            terminal_selection: None,
            workspace_surface: if terminal
                .is_some_and(|terminal| terminal.active_screen() == ScreenBuffer::Alternate)
            {
                WorkspaceSurfaceKind::Terminal
            } else {
                WorkspaceSurfaceKind::Agent
            },
            file_editor_host: &file_editor_host,
            file_editor_prompt: crate::file_editor_pane::FileEditorPrompt::None,
            file_editor_search: &crate::file_editor_search::FileEditorSearchState::default(),
            file_editor_diagnostics: &[],
            language_hover: None,
            language_completions: None,
            completion_selection: 0,
            code_editor_style: &code_editor_style,
            thread_projection: &thread_projection,
            thread_timeline_scroll_offset: 0,
            workspace_context: &workspace_context,
            composer: &composer,
            composer_interaction: &composer_interaction,
            composer_interaction_pane: &composer_interaction_pane,
            composer_mode: ComposerMode::Agent,
            session_search: &session_search,
            caret_visibility: CaretVisibility::Visible,
            dispatch: &dispatch,
            session_sidebar,
            agent_sidebar,
            agent_sidebar_workspace: &agent_sidebar_workspace,
            session_context_menu,
            git_branch_context_menu: &GitBranchContextMenuState::default(),
            workspace_path_picker: &WorkspacePathPickerState::default(),
            keybindings: &NativeKeybindings::default(),
            keyboard_shortcuts: &KeyboardShortcutsState::default(),
            language_server_settings: &LanguageServerSettingsState::default(),
            language_server_runtime_state: None,
            keybinding_diagnostics: &[],
            window_control_insets: WindowControlInsets::NONE,
            pointer_position: None,
        },
        &mut text_layout,
    )
}

#[test]
fn primary_layout_keeps_output_above_a_bottom_composer() {
    let layout = ShellLayout::for_viewport(
        viewport(),
        SessionSidebarState::default(),
        AgentSidebarState::default(),
    )
    .unwrap();

    assert_eq!(layout.titlebar.origin.y, 0.0);
    assert_eq!(layout.titlebar.size.height, 32.0);
    assert_eq!(layout.main.origin.x, 0.0);
    assert_eq!(layout.main.bottom(), 700.0);
    assert_eq!(layout.output.bottom(), layout.composer_panel.origin.y);
    assert_eq!(layout.composer_panel.origin.y, 572.0);
    assert_eq!(layout.composer_info_bar.origin.y, 580.0);
    assert_eq!(layout.composer.origin.y, 612.0);
    assert_eq!(layout.composer.bottom(), 656.0);
    assert_eq!(layout.composer_toolbar.origin.y, 664.0);
}

#[test]
fn editor_surface_mounts_the_active_file_without_agent_composer_or_timeline() {
    let composer = ComposerEditor::default();
    let composer_interaction = ComposerInteractionModel::new();
    let composer_interaction_pane = ComposerInteractionPaneState::default();
    let session_search = SessionSearch::default();
    let workspace_context = WorkspaceContext::fixture("~/Desktop/zeta", Some("main"), Some(0));
    let agent_sidebar_workspace = AgentSidebarWorkspace::default();
    let thread_projection = ThreadProjection::default();
    let mut file_editor_host = FileEditorHost::default();
    file_editor_host.open(TextFileSnapshot::new(
        "src/main.rs".into(),
        "fn main() {}\n".into(),
        TextFileDiskVersion::new(
            13,
            TextFileModifiedAt::KnownMillis(1),
            TextFileAccess::Writable,
        ),
    ));
    let code_editor_style = CodeEditorStyle::light();
    let dispatch = UiDispatch::default();
    let mut text_layout = TextInputLayoutEngine::new();

    let presentation = build_shell_presentation(
        viewport(),
        ShellPresentationModel {
            palette: crate::shell_style::SHELL_PALETTE,
            terminal: None,
            terminal_scroll_offset: 0,
            terminal_scrollbar_presentation: ScrollbarPresentation::default(),
            terminal_selection: None,
            workspace_surface: WorkspaceSurfaceKind::Editor,
            file_editor_host: &file_editor_host,
            file_editor_prompt: crate::file_editor_pane::FileEditorPrompt::None,
            file_editor_search: &crate::file_editor_search::FileEditorSearchState::default(),
            file_editor_diagnostics: &[],
            language_hover: None,
            language_completions: None,
            completion_selection: 0,
            code_editor_style: &code_editor_style,
            thread_projection: &thread_projection,
            thread_timeline_scroll_offset: 0,
            workspace_context: &workspace_context,
            composer: &composer,
            composer_interaction: &composer_interaction,
            composer_interaction_pane: &composer_interaction_pane,
            composer_mode: ComposerMode::Agent,
            session_search: &session_search,
            caret_visibility: CaretVisibility::Visible,
            dispatch: &dispatch,
            session_sidebar: SessionSidebarState::default(),
            agent_sidebar: AgentSidebarState::default(),
            agent_sidebar_workspace: &agent_sidebar_workspace,
            session_context_menu: SessionContextMenuState::default(),
            git_branch_context_menu: &GitBranchContextMenuState::default(),
            workspace_path_picker: &WorkspacePathPickerState::default(),
            keybindings: &NativeKeybindings::default(),
            keyboard_shortcuts: &KeyboardShortcutsState::default(),
            language_server_settings: &LanguageServerSettingsState::default(),
            language_server_runtime_state: None,
            keybinding_diagnostics: &[],
            window_control_insets: WindowControlInsets::NONE,
            pointer_position: None,
        },
        &mut text_layout,
    );

    for id in [FILE_EDITOR_PANE, FILE_EDITOR_TAB_LIST, FILE_EDITOR_DOCUMENT] {
        assert!(
            presentation
                .accessibility_nodes
                .iter()
                .any(|node| node.id == id)
        );
    }
    assert!(
        presentation
            .accessibility_nodes
            .iter()
            .all(|node| node.id != COMPOSER && node.id != THREAD_TIMELINE)
    );
    assert!(
        presentation
            .scene
            .text_blocks()
            .iter()
            .any(|block| block.text() == "fn main() {}")
    );
}

#[test]
fn multiline_composer_grows_upward_between_info_bar_and_bottom_toolbar() {
    let layout = ShellLayout::for_viewport_with_composer_height(
        viewport(),
        SessionSidebarState::default(),
        AgentSidebarState::default(),
        160.0,
    )
    .unwrap();

    assert_eq!(layout.composer.size.height, 160.0);
    assert_eq!(layout.composer_panel.size.height, 244.0);
    assert_eq!(
        layout.composer.origin.y,
        layout.composer_info_bar.bottom() + 8.0
    );
    assert_eq!(
        layout.composer.bottom() + 8.0,
        layout.composer_toolbar.origin.y
    );
    assert_eq!(layout.output.bottom(), layout.composer_panel.origin.y);
}

#[test]
fn primary_presentation_uses_a_flat_light_surface() {
    let layout = ShellLayout::for_viewport(
        viewport(),
        SessionSidebarState::default(),
        AgentSidebarState::default(),
    )
    .unwrap();
    let presentation = presentation(None, 0);
    let composer_panel = presentation
        .scene
        .rects()
        .iter()
        .find(|rect| rect.bounds() == layout.composer_panel)
        .unwrap();
    let info_editor_separator = presentation
        .scene
        .rects()
        .iter()
        .find(|rect| rect.bounds() == layout.composer_panel_layout.info_editor_separator())
        .unwrap();

    assert_eq!(presentation.scene.background(), Color::rgb(252, 252, 253));
    assert_eq!(composer_panel.fill(), Color::WHITE);
    assert_eq!(composer_panel.border().widths().top, 1.0);
    assert_eq!(
        info_editor_separator.fill(),
        crate::shell_style::SHELL_PALETTE.border
    );
    assert!(
        presentation
            .scene
            .rects()
            .iter()
            .all(|rect| rect.corner_radii().top_left <= 4.0)
    );
}

#[test]
fn primary_presentation_has_an_agent_timeline_and_fixed_composer() {
    let presentation = presentation(None, 0);
    let visible_text = presentation
        .scene
        .text_blocks()
        .iter()
        .map(|block| block.text())
        .collect::<Vec<_>>();

    assert!(!visible_text.contains(&"zeterm"));
    assert!(!visible_text.contains(&"Starting shell…"));
    assert!(visible_text.contains(&"Ask Zeta anything…"));
    assert!(visible_text.contains(&"Agent"));
    assert!(!visible_text.contains(&"SESSIONS"));
    assert_eq!(presentation.scene.icons().len(), 8);
}

#[test]
fn expanded_sidebar_reflows_the_terminal_and_publishes_a_selected_session_tab() {
    let layout = ShellLayout::for_viewport(
        viewport(),
        SessionSidebarState::expanded(),
        AgentSidebarState::default(),
    )
    .unwrap();
    let presentation = presentation_with_sidebar(None, 0, SessionSidebarState::expanded());
    let visible_text = presentation
        .scene
        .text_blocks()
        .iter()
        .map(|block| block.text())
        .collect::<Vec<_>>();
    let session_tab = presentation
        .accessibility_nodes
        .iter()
        .find(|node| node.id == crate::shell_interaction::ACTIVE_SESSION_TAB)
        .unwrap();
    let resize_handle = presentation
        .accessibility_nodes
        .iter()
        .find(|node| node.id == SESSION_SIDEBAR_RESIZE_HANDLE)
        .unwrap();
    let search = presentation
        .accessibility_nodes
        .iter()
        .find(|node| node.id == SESSION_SEARCH_INPUT)
        .unwrap();
    let add_session = presentation
        .accessibility_nodes
        .iter()
        .find(|node| node.id == ADD_SESSION)
        .unwrap();
    let mut dispatch = UiDispatch::default();

    assert_eq!(layout.session_sidebar.unwrap().size.width, 200.0);
    assert_eq!(layout.main.origin.x, 200.0);
    assert_eq!(layout.composer.origin.x, 224.0);
    assert!(visible_text.contains(&"Search sessions..."));
    let inspected_search = presentation
        .scene
        .inspection()
        .target_at(Point::new(20.0, 50.0))
        .expect("session search should expose its inspection hierarchy");
    assert_eq!(
        presentation
            .scene
            .inspection()
            .ancestry(inspected_search.id())
            .iter()
            .map(|node| node.name())
            .collect::<Vec<_>>(),
        vec![
            "SessionSidebar",
            "SessionSidebarToolbar",
            "SearchBox",
            "InputBox"
        ]
    );
    assert_eq!(search.role, AccessibilityRole::TextInput);
    assert_eq!(add_session.role, AccessibilityRole::Button);
    assert_eq!(add_session.label, "Add new session");
    assert_eq!(session_tab.role, AccessibilityRole::Tab);
    assert_eq!(session_tab.selection, zui::AccessibilitySelection::Selected);
    assert_eq!(resize_handle.role, AccessibilityRole::Separator);
    assert_eq!(resize_handle.label, "Resize sessions sidebar");
    assert_eq!(resize_handle.value.as_deref(), Some("200 pixels"));
    assert_eq!(
        presentation
            .interaction_frame
            .target_at(Point::new(200.0, 100.0)),
        Some(SESSION_SIDEBAR_RESIZE_HANDLE)
    );
    dispatch.pointer_moved(Point::new(200.0, 100.0), &presentation.interaction_frame);
    assert_eq!(
        dispatch.pointer_feedback(&presentation.interaction_frame),
        CursorFeedback::ResizeHorizontal
    );
    assert_eq!(
        terminal_grid_size_for_viewport(
            viewport(),
            ScreenBuffer::Primary,
            SessionSidebarState::expanded(),
            AgentSidebarState::default(),
        )
        .cols(),
        94
    );
    assert_eq!(
        terminal_mouse_position_for_viewport(
            viewport(),
            ScreenBuffer::Primary,
            SessionSidebarState::expanded(),
            AgentSidebarState::default(),
            Point::new(100.0, 100.0),
        ),
        None
    );
}

#[test]
fn session_search_filters_tabs_by_session_name() {
    let composer = ComposerEditor::default();
    let mut session_search = SessionSearch::default();
    session_search.apply(TextInputCommand::Insert("missing session".to_owned()));
    let workspace_context = WorkspaceContext::fixture("~/Desktop/zeta", Some("main"), Some(0));
    let mut text_layout = TextInputLayoutEngine::new();
    let dispatch = UiDispatch::default();
    let agent_sidebar_workspace = AgentSidebarWorkspace::default();
    let file_editor_host = FileEditorHost::default();
    let code_editor_style = CodeEditorStyle::light();
    let thread_projection = ThreadProjection::default();

    let presentation = build_shell_presentation(
        viewport(),
        ShellPresentationModel {
            palette: crate::shell_style::SHELL_PALETTE,
            terminal: None,
            terminal_scroll_offset: 0,
            terminal_scrollbar_presentation: ScrollbarPresentation::default(),
            terminal_selection: None,
            workspace_surface: WorkspaceSurfaceKind::Agent,
            file_editor_host: &file_editor_host,
            file_editor_prompt: crate::file_editor_pane::FileEditorPrompt::None,
            file_editor_search: &crate::file_editor_search::FileEditorSearchState::default(),
            file_editor_diagnostics: &[],
            language_hover: None,
            language_completions: None,
            completion_selection: 0,
            code_editor_style: &code_editor_style,
            thread_projection: &thread_projection,
            thread_timeline_scroll_offset: 0,
            workspace_context: &workspace_context,
            composer: &composer,
            composer_interaction: &ComposerInteractionModel::new(),
            composer_interaction_pane: &ComposerInteractionPaneState::default(),
            composer_mode: ComposerMode::Agent,
            session_search: &session_search,
            caret_visibility: CaretVisibility::Visible,
            dispatch: &dispatch,
            session_sidebar: SessionSidebarState::expanded(),
            agent_sidebar: AgentSidebarState::default(),
            agent_sidebar_workspace: &agent_sidebar_workspace,
            session_context_menu: SessionContextMenuState::default(),
            git_branch_context_menu: &GitBranchContextMenuState::default(),
            workspace_path_picker: &WorkspacePathPickerState::default(),
            keybindings: &NativeKeybindings::default(),
            keyboard_shortcuts: &KeyboardShortcutsState::default(),
            language_server_settings: &LanguageServerSettingsState::default(),
            language_server_runtime_state: None,
            keybinding_diagnostics: &[],
            window_control_insets: WindowControlInsets::NONE,
            pointer_position: None,
        },
        &mut text_layout,
    );

    assert!(
        presentation
            .accessibility_nodes
            .iter()
            .all(|node| node.id != crate::shell_interaction::ACTIVE_SESSION_TAB)
    );
    assert!(
        presentation
            .scene
            .text_blocks()
            .iter()
            .any(|block| block.text() == "missing session")
    );
}

#[test]
fn expanded_agent_sidebar_defaults_to_the_files_pane_with_navigation_and_actions() {
    let agent_sidebar = AgentSidebarState::expanded();
    let layout =
        ShellLayout::for_viewport(viewport(), SessionSidebarState::default(), agent_sidebar)
            .unwrap();
    let presentation = presentation_with_sidebars_and_menu(
        None,
        0,
        SessionSidebarState::default(),
        agent_sidebar,
        SessionContextMenuState::default(),
    );
    let sidebar = presentation
        .accessibility_nodes
        .iter()
        .find(|node| node.id == AGENT_SIDEBAR)
        .unwrap();
    let explorer = presentation
        .accessibility_nodes
        .iter()
        .find(|node| node.id == AGENT_EXPLORER_PANE)
        .unwrap();
    let navigation = presentation
        .accessibility_nodes
        .iter()
        .find(|node| node.id == AGENT_SIDEBAR_NAVIGATION)
        .unwrap();
    let toolbar = presentation
        .accessibility_nodes
        .iter()
        .find(|node| node.id == AGENT_SIDEBAR_TOOLBAR)
        .unwrap();
    let resize_handle = presentation
        .accessibility_nodes
        .iter()
        .find(|node| node.id == AGENT_SIDEBAR_RESIZE_HANDLE)
        .unwrap();

    assert_eq!(
        layout.agent_sidebar,
        Some(zeta_ui::Rect::from_xywh(680.0, 32.0, 320.0, 668.0))
    );
    assert_eq!(
        layout.main,
        zeta_ui::Rect::from_xywh(0.0, 32.0, 680.0, 668.0)
    );
    assert_eq!(sidebar.role, AccessibilityRole::Group);
    assert_eq!(sidebar.label, "Agent sidebar");
    assert_eq!(explorer.parent, Some(AGENT_SIDEBAR));
    assert_eq!(explorer.label, "Files");
    assert_eq!(navigation.role, AccessibilityRole::Toolbar);
    assert_eq!(toolbar.label, "Agent sidebar toolbar");
    assert_eq!(resize_handle.role, AccessibilityRole::Separator);
    assert_eq!(resize_handle.label, "Resize agent sidebar");
    assert_eq!(resize_handle.value.as_deref(), Some("680 pixels"));
    assert_eq!(
        resize_handle.bounds,
        zeta_ui::Rect::from_xywh(676.0, 32.0, 8.0, 668.0)
    );
    let mut resize_dispatch = UiDispatch::default();
    resize_dispatch.pointer_moved(Point::new(680.0, 100.0), &presentation.interaction_frame);
    assert_eq!(
        resize_dispatch.pointer_feedback(&presentation.interaction_frame),
        CursorFeedback::ResizeHorizontal
    );
    assert_eq!(
        toolbar.bounds,
        zeta_ui::Rect::from_xywh(680.0, 32.0, 320.0, 36.0)
    );
    assert_eq!(
        navigation.bounds,
        zeta_ui::Rect::from_xywh(680.0, 32.0, 128.0, 36.0)
    );
    assert_eq!(navigation.parent, Some(AGENT_SIDEBAR_TOOLBAR));
    assert_eq!(
        explorer.bounds,
        zeta_ui::Rect::from_xywh(680.0, 68.0, 320.0, 632.0)
    );
    for id in [
        AGENT_CHANGES,
        AGENT_FILES,
        AGENT_FILES_REFRESH,
        AGENT_FILES_SEARCH,
    ] {
        assert!(
            presentation
                .accessibility_nodes
                .iter()
                .any(|node| node.id == id)
        );
    }
    assert!(
        presentation
            .accessibility_nodes
            .iter()
            .all(|node| !matches!(node.id, AGENT_EDITOR_PANE | MULTI_DIFF_EDITOR))
    );
    let visible_text = presentation
        .scene
        .text_blocks()
        .iter()
        .map(|text| text.text())
        .collect::<Vec<_>>();
    assert_eq!(
        visible_text.iter().filter(|text| **text == "Files").count(),
        1
    );
    assert!(visible_text.contains(&"No files loaded"));
    assert!(visible_text.contains(&"↑0 ↓0"));
    assert_eq!(
        presentation
            .accessibility_nodes
            .iter()
            .filter(|node| node.parent == Some(crate::shell_interaction::AGENT_FILES_ACTION_BAR))
            .count(),
        2
    );
    assert_eq!(
        terminal_grid_size_for_viewport(
            viewport(),
            ScreenBuffer::Primary,
            SessionSidebarState::default(),
            agent_sidebar,
        )
        .cols(),
        79
    );
}

#[test]
fn changes_switch_mounts_workspace_diffs_in_the_multi_diff_editor_without_files_actions() {
    let composer = ComposerEditor::default();
    let session_search = SessionSearch::default();
    let workspace_context = WorkspaceContext::fixture("~/Desktop/zeta", Some("main"), Some(2));
    let mut agent_workspace = AgentSidebarWorkspace::default();
    agent_workspace.sync_repository(&workspace_context);
    agent_workspace.select_view(AgentSidebarView::Changes);
    let mut text_layout = TextInputLayoutEngine::new();
    let dispatch = UiDispatch::default();
    let thread_projection = ThreadProjection::default();
    let file_editor_host = FileEditorHost::default();
    let code_editor_style = CodeEditorStyle::light();
    let presentation = build_shell_presentation(
        viewport(),
        ShellPresentationModel {
            palette: crate::shell_style::SHELL_PALETTE,
            terminal: None,
            terminal_scroll_offset: 0,
            terminal_scrollbar_presentation: ScrollbarPresentation::default(),
            terminal_selection: None,
            workspace_surface: WorkspaceSurfaceKind::Agent,
            file_editor_host: &file_editor_host,
            file_editor_prompt: crate::file_editor_pane::FileEditorPrompt::None,
            file_editor_search: &crate::file_editor_search::FileEditorSearchState::default(),
            file_editor_diagnostics: &[],
            language_hover: None,
            language_completions: None,
            completion_selection: 0,
            code_editor_style: &code_editor_style,
            thread_projection: &thread_projection,
            thread_timeline_scroll_offset: 0,
            workspace_context: &workspace_context,
            composer: &composer,
            composer_interaction: &ComposerInteractionModel::new(),
            composer_interaction_pane: &ComposerInteractionPaneState::default(),
            composer_mode: ComposerMode::Agent,
            session_search: &session_search,
            caret_visibility: CaretVisibility::Visible,
            dispatch: &dispatch,
            session_sidebar: SessionSidebarState::default(),
            agent_sidebar: AgentSidebarState::expanded(),
            agent_sidebar_workspace: &agent_workspace,
            session_context_menu: SessionContextMenuState::default(),
            git_branch_context_menu: &GitBranchContextMenuState::default(),
            workspace_path_picker: &WorkspacePathPickerState::default(),
            keybindings: &NativeKeybindings::default(),
            keyboard_shortcuts: &KeyboardShortcutsState::default(),
            language_server_settings: &LanguageServerSettingsState::default(),
            language_server_runtime_state: None,
            keybinding_diagnostics: &[],
            window_control_insets: WindowControlInsets::NONE,
            pointer_position: None,
        },
        &mut text_layout,
    );

    assert!(
        presentation
            .accessibility_nodes
            .iter()
            .any(|node| node.id == AGENT_EDITOR_PANE)
    );
    assert!(
        presentation
            .accessibility_nodes
            .iter()
            .any(|node| node.id == MULTI_DIFF_EDITOR)
    );
    assert!(
        presentation
            .accessibility_nodes
            .iter()
            .all(|node| !matches!(
                node.id,
                AGENT_EXPLORER_PANE | AGENT_FILES_REFRESH | AGENT_FILES_SEARCH
            ))
    );
    let visible_text = presentation
        .scene
        .text_blocks()
        .iter()
        .map(|text| text.text())
        .collect::<Vec<_>>();
    assert!(visible_text.contains(&"fixture-0.txt"));
    assert!(visible_text.contains(&"fixture-1.txt"));
    assert!(!visible_text.contains(&"No changed files"));
    assert!(!visible_text.contains(&"HEAD"));
    assert!(!visible_text.contains(&"Working Tree"));
    assert_eq!(
        visible_text
            .iter()
            .filter(|text| **text == "Changes")
            .count(),
        1
    );
}

#[test]
fn open_session_context_menu_is_topmost_and_exposes_four_actions() {
    let mut menu_state = SessionContextMenuState::default();
    menu_state.open(
        crate::shell_interaction::ACTIVE_SESSION_TAB,
        Point::new(80.0, 120.0),
        Some(COMPOSER),
    );
    let presentation =
        presentation_with_sidebar_and_menu(None, 0, SessionSidebarState::expanded(), menu_state);
    let labels = presentation
        .accessibility_nodes
        .iter()
        .filter(|node| node.parent == Some(crate::shell_interaction::SESSION_CONTEXT_MENU))
        .map(|node| node.label.as_str())
        .collect::<Vec<_>>();
    let first_item = presentation
        .accessibility_nodes
        .iter()
        .find(|node| {
            node.id == crate::shell_interaction::SessionContextMenuAction::Pin.element_id()
        })
        .unwrap();

    assert_eq!(labels, ["Pin", "Close", "Rename", "Fork"]);
    assert_eq!(
        presentation.interaction_frame.target_at(Point::new(
            first_item.bounds.origin.x + 2.0,
            first_item.bounds.origin.y + 2.0
        )),
        Some(crate::shell_interaction::SessionContextMenuAction::Pin.element_id())
    );
    assert!(
        presentation
            .scene
            .text_blocks()
            .iter()
            .any(|text| text.text() == "Fork")
    );
}

#[test]
fn primary_presentation_publishes_current_control_semantics_and_focus() {
    let presentation = presentation(None, 0);
    let info_bar = presentation
        .accessibility_nodes
        .iter()
        .find(|node| node.id == COMPOSER_INFO_BAR)
        .unwrap();
    let composer = presentation
        .accessibility_nodes
        .iter()
        .find(|node| node.id == COMPOSER)
        .unwrap();
    let location = presentation
        .accessibility_nodes
        .iter()
        .find(|node| node.id == ContextAction::Location.element_id())
        .unwrap();

    assert_eq!(info_bar.role, AccessibilityRole::Group);
    assert_eq!(info_bar.label, "/ for commands");
    let inspected_info_bar = presentation
        .scene
        .inspection()
        .target_at(Point::new(
            info_bar.bounds.origin.x + 100.0,
            info_bar.bounds.origin.y + info_bar.bounds.size.height / 2.0,
        ))
        .expect("composer info bar should expose its inspection hierarchy");
    assert_eq!(
        presentation
            .scene
            .inspection()
            .ancestry(inspected_info_bar.id())
            .iter()
            .map(|node| node.name())
            .collect::<Vec<_>>(),
        vec!["ComposerPanel", "ComposerInfoBar"]
    );
    assert_eq!(composer.role, AccessibilityRole::TextInput);
    assert_eq!(composer.label, "Command input");
    assert_eq!(composer.value.as_deref(), Some(""));
    assert!(composer.focused);
    assert_eq!(location.role, AccessibilityRole::Button);
    assert_eq!(location.label, "Environment: Local");
    assert!(!location.focused);
}

#[test]
fn context_toolbar_pointer_clicks_activate_workspace_and_branch_pickers() {
    let presentation = presentation(None, 0);

    for action in [ContextAction::WorkingDirectory, ContextAction::GitBranch] {
        let bounds = presentation
            .accessibility_nodes
            .iter()
            .find(|node| node.id == action.element_id())
            .unwrap()
            .bounds;
        let point = Point::new(
            bounds.origin.x + bounds.size.width / 2.0,
            bounds.origin.y + bounds.size.height / 2.0,
        );
        let mut dispatch = UiDispatch::default();

        dispatch.pointer_moved(point, &presentation.interaction_frame);
        dispatch.press_primary(&presentation.interaction_frame);
        let outcome = dispatch.release_primary(point, &presentation.interaction_frame);

        assert_eq!(
            outcome.intent,
            Some(UiIntent::Activate(action.element_id()))
        );
    }
}

#[test]
fn overlay_rebuild_restores_the_retained_base_scene_and_interactions() {
    let composer = ComposerEditor::default();
    let composer_interaction = ComposerInteractionModel::new();
    let composer_interaction_pane = ComposerInteractionPaneState::default();
    let session_search = SessionSearch::default();
    let workspace_context = WorkspaceContext::fixture("~/Desktop/zeta", Some("main"), Some(0));
    let agent_sidebar_workspace = AgentSidebarWorkspace::default();
    let thread_projection = ThreadProjection::default();
    let git_branch_context_menu = GitBranchContextMenuState::default();
    let workspace_path_picker = WorkspacePathPickerState::default();
    let keybindings = NativeKeybindings::default();
    let keyboard_shortcuts = KeyboardShortcutsState::default();
    let language_server_settings = LanguageServerSettingsState::default();
    let file_editor_host = FileEditorHost::default();
    let code_editor_style = CodeEditorStyle::light();
    let dispatch = UiDispatch::default();
    let mut text_layout = TextInputLayoutEngine::new();
    let closed_model = ShellPresentationModel {
        palette: crate::shell_style::SHELL_PALETTE,
        terminal: None,
        terminal_scroll_offset: 0,
        terminal_scrollbar_presentation: ScrollbarPresentation::default(),
        terminal_selection: None,
        workspace_surface: WorkspaceSurfaceKind::Agent,
        file_editor_host: &file_editor_host,
        file_editor_prompt: crate::file_editor_pane::FileEditorPrompt::None,
        file_editor_search: &crate::file_editor_search::FileEditorSearchState::default(),
        file_editor_diagnostics: &[],
        language_hover: None,
        language_completions: None,
        completion_selection: 0,
        code_editor_style: &code_editor_style,
        thread_projection: &thread_projection,
        thread_timeline_scroll_offset: 0,
        workspace_context: &workspace_context,
        composer: &composer,
        composer_interaction: &composer_interaction,
        composer_interaction_pane: &composer_interaction_pane,
        composer_mode: ComposerMode::Agent,
        session_search: &session_search,
        caret_visibility: CaretVisibility::Visible,
        dispatch: &dispatch,
        session_sidebar: SessionSidebarState::default(),
        agent_sidebar: AgentSidebarState::default(),
        agent_sidebar_workspace: &agent_sidebar_workspace,
        session_context_menu: SessionContextMenuState::default(),
        git_branch_context_menu: &git_branch_context_menu,
        workspace_path_picker: &workspace_path_picker,
        keybindings: &keybindings,
        keyboard_shortcuts: &keyboard_shortcuts,
        language_server_settings: &language_server_settings,
        language_server_runtime_state: None,
        keybinding_diagnostics: &[],
        window_control_insets: WindowControlInsets::NONE,
        pointer_position: None,
    };
    let mut presentation = build_shell_presentation(viewport(), closed_model, &mut text_layout);
    let base_scene = presentation.scene.clone();
    let base_interactions = presentation.interaction_frame.clone();
    let base_accessibility = presentation.accessibility_nodes.clone();
    let mut menu = SessionContextMenuState::default();
    menu.open(ACTIVE_SESSION_TAB, Point::new(200.0, 100.0), None);

    assert!(rebuild_shell_overlays(
        &mut presentation,
        viewport(),
        ShellPresentationModel {
            session_context_menu: menu,
            ..closed_model
        },
        &mut text_layout,
    ));
    assert!(
        presentation
            .interaction_frame
            .node(SESSION_CONTEXT_MENU)
            .is_some()
    );
    assert_ne!(presentation.scene, base_scene);

    assert!(rebuild_shell_overlays(
        &mut presentation,
        viewport(),
        closed_model,
        &mut text_layout,
    ));
    assert_eq!(presentation.scene, base_scene);
    assert_eq!(presentation.interaction_frame, base_interactions);
    assert_eq!(presentation.accessibility_nodes, base_accessibility);
}

#[test]
fn titlebar_drags_the_window_and_composer_is_a_registered_input_region() {
    let presentation = presentation(None, 0);
    let mut dispatch = UiDispatch::default();

    assert_eq!(
        dispatch
            .pointer_moved(Point::new(500.0, 17.0), &presentation.interaction_frame)
            .invalidation,
        DispatchInvalidation::Paint
    );
    assert_eq!(
        dispatch
            .press_primary(&presentation.interaction_frame)
            .intent,
        Some(UiIntent::StartWindowDrag(TITLEBAR))
    );
    assert_eq!(
        dispatch
            .pointer_moved(Point::new(500.0, 640.0), &presentation.interaction_frame)
            .invalidation,
        DispatchInvalidation::Paint
    );
    assert_eq!(
        presentation
            .interaction_frame
            .target_at(Point::new(500.0, 640.0)),
        Some(COMPOSER)
    );
    assert_eq!(
        presentation
            .interaction_frame
            .target_at(Point::new(28.0, 688.0)),
        Some(COMPOSER_PANEL)
    );
    assert_eq!(
        dispatch.pointer_feedback(&presentation.interaction_frame),
        CursorFeedback::Text
    );
}

#[test]
fn context_toolbar_registers_button_geometry_below_the_composer_editor() {
    let presentation = presentation(None, 0);
    let mut dispatch = UiDispatch::default();

    assert_eq!(
        dispatch
            .pointer_moved(Point::new(40.0, 676.0), &presentation.interaction_frame)
            .invalidation,
        DispatchInvalidation::Paint
    );
    assert_eq!(
        presentation
            .interaction_frame
            .target_at(Point::new(40.0, 676.0)),
        Some(COMPOSER_MODE)
    );
    assert_eq!(
        dispatch.pointer_feedback(&presentation.interaction_frame),
        CursorFeedback::Pointer
    );
    assert_eq!(
        dispatch
            .press_primary(&presentation.interaction_frame)
            .invalidation,
        DispatchInvalidation::Paint
    );
    assert!(dispatch.is_pressed(COMPOSER_MODE));
}

#[test]
fn compact_viewport_uses_bounded_fallback_scene() {
    let composer = ComposerEditor::default();
    let session_search = SessionSearch::default();
    let workspace_context = WorkspaceContext::fixture("/tmp/project", None, None);
    let mut text_layout = TextInputLayoutEngine::new();
    let dispatch = UiDispatch::default();
    let agent_sidebar_workspace = AgentSidebarWorkspace::default();
    let thread_projection = ThreadProjection::default();
    let file_editor_host = FileEditorHost::default();
    let code_editor_style = CodeEditorStyle::light();
    let presentation = build_shell_presentation(
        LogicalViewport {
            width: 220.0,
            height: 100.0,
        },
        ShellPresentationModel {
            palette: crate::shell_style::SHELL_PALETTE,
            terminal: None,
            terminal_scroll_offset: 0,
            terminal_scrollbar_presentation: ScrollbarPresentation::default(),
            terminal_selection: None,
            workspace_surface: WorkspaceSurfaceKind::Agent,
            file_editor_host: &file_editor_host,
            file_editor_prompt: crate::file_editor_pane::FileEditorPrompt::None,
            file_editor_search: &crate::file_editor_search::FileEditorSearchState::default(),
            file_editor_diagnostics: &[],
            language_hover: None,
            language_completions: None,
            completion_selection: 0,
            code_editor_style: &code_editor_style,
            thread_projection: &thread_projection,
            thread_timeline_scroll_offset: 0,
            workspace_context: &workspace_context,
            composer: &composer,
            composer_interaction: &ComposerInteractionModel::new(),
            composer_interaction_pane: &ComposerInteractionPaneState::default(),
            composer_mode: ComposerMode::Agent,
            session_search: &session_search,
            caret_visibility: CaretVisibility::Visible,
            dispatch: &dispatch,
            session_sidebar: SessionSidebarState::default(),
            agent_sidebar: AgentSidebarState::default(),
            agent_sidebar_workspace: &agent_sidebar_workspace,
            session_context_menu: SessionContextMenuState::default(),
            git_branch_context_menu: &GitBranchContextMenuState::default(),
            workspace_path_picker: &WorkspacePathPickerState::default(),
            keybindings: &NativeKeybindings::default(),
            keyboard_shortcuts: &KeyboardShortcutsState::default(),
            language_server_settings: &LanguageServerSettingsState::default(),
            language_server_runtime_state: None,
            keybinding_diagnostics: &[],
            window_control_insets: WindowControlInsets::NONE,
            pointer_position: None,
        },
        &mut text_layout,
    );

    assert_eq!(presentation.scene.rects().len(), 1);
    assert_eq!(presentation.scene.text_blocks().len(), 1);
    assert_eq!(presentation.scene.text_blocks()[0].text(), "zeterm");
}

#[test]
fn primary_reserves_rows_for_composer_while_alternate_screen_uses_full_height() {
    let primary = terminal_grid_size_for_viewport(
        viewport(),
        ScreenBuffer::Primary,
        SessionSidebarState::default(),
        AgentSidebarState::default(),
    );
    let alternate = terminal_grid_size_for_viewport(
        viewport(),
        ScreenBuffer::Alternate,
        SessionSidebarState::default(),
        AgentSidebarState::default(),
    );

    assert_eq!(primary, GridSize::new(27, 119));
    assert_eq!(alternate, GridSize::new(34, 119));
}

#[test]
fn primary_pointer_coordinates_are_limited_to_the_output_region() {
    let first = terminal_mouse_position_for_viewport(
        viewport(),
        ScreenBuffer::Primary,
        SessionSidebarState::default(),
        AgentSidebarState::default(),
        Point::new(24.0, 60.0),
    )
    .unwrap();
    let composer = terminal_mouse_position_for_viewport(
        viewport(),
        ScreenBuffer::Primary,
        SessionSidebarState::default(),
        AgentSidebarState::default(),
        Point::new(40.0, 640.0),
    );

    assert_eq!((first.row(), first.col()), (0, 0));
    assert_eq!(composer, None);
}

#[test]
fn primary_terminal_blocks_do_not_override_the_agent_timeline() {
    let mut terminal = TerminalCore::new(GridSize::new(29, 119));
    terminal.process_output(b"$ ");
    terminal.start_command("printf hi");
    terminal.process_output(b"\x1b[32mhi\x1b[0m\r\n");

    let presentation = presentation(Some(&terminal), 0);
    let visible_text = presentation
        .scene
        .text_blocks()
        .iter()
        .map(|block| block.text())
        .collect::<Vec<_>>();

    assert!(!visible_text.contains(&"❯ printf hi"));
    assert!(!visible_text.contains(&"hi"));
    assert!(visible_text.contains(&"Ask Zeta anything…"));
}

#[test]
fn primary_terminal_scrollback_does_not_change_the_agent_timeline() {
    let mut terminal = TerminalCore::new(GridSize::new(29, 119));
    terminal.start_command("history");
    for index in 0..80 {
        terminal.process_output(format!("line-{index}\r\n").as_bytes());
    }
    let presentation = presentation(Some(&terminal), 80);
    let visible_text = presentation
        .scene
        .text_blocks()
        .iter()
        .map(|block| block.text())
        .collect::<Vec<_>>();

    assert!(!visible_text.contains(&"❯ history"));
    assert!(!visible_text.contains(&"line-0"));
    assert!(!visible_text.contains(&"line-79"));
    assert!(visible_text.contains(&"Ask Zeta anything…"));
}

#[test]
fn primary_ime_candidate_position_comes_from_the_bottom_composer() {
    let terminal = TerminalCore::new(GridSize::new(29, 119));
    let layout = ShellLayout::for_viewport(
        viewport(),
        SessionSidebarState::default(),
        AgentSidebarState::default(),
    )
    .unwrap();

    let presentation = presentation(Some(&terminal), 0);
    let caret = presentation.ime_cursor_area.unwrap();

    assert!(layout.composer.contains(caret.origin));
}

#[test]
fn alternate_screen_ime_position_comes_from_the_terminal_cursor() {
    let mut terminal = TerminalCore::new(GridSize::new(34, 119));
    terminal.process_output(b"\x1b[?1049habc");

    let presentation = presentation(Some(&terminal), 0);
    let caret = presentation.ime_cursor_area.unwrap();

    assert_eq!(caret.origin, Point::new(48.0, 56.0));
    assert_eq!(caret.size.width, 8.0);
    assert_eq!(caret.size.height, 18.0);
}

#[test]
fn background_terminal_title_does_not_replace_the_agent_session_title() {
    let mut terminal = TerminalCore::new(GridSize::new(29, 119));
    terminal.process_output(b"\x1b]2;project shell\x07");

    let presentation =
        presentation_with_sidebar(Some(&terminal), 0, SessionSidebarState::expanded());

    let text = presentation
        .scene
        .text_blocks()
        .iter()
        .map(|block| block.text())
        .collect::<Vec<_>>();
    assert!(text.contains(&PRODUCT_DISPLAY_NAME));
    assert!(!text.contains(&"project shell"));
}
