use super::{
    LogicalViewport, ShellLayout, ShellPresentation, ShellPresentationModel,
    build_shell_presentation, rebuild_shell_overlays, terminal_grid_size_for_viewport,
    terminal_mouse_position_for_viewport, terminal_pane_sash_for_viewport,
};
use crate::PRODUCT_DISPLAY_NAME;
use crate::file_editor_host::FileEditorHost;
use crate::git_branch_context_menu::GitBranchContextMenuState;
use crate::keybindings::NativeKeybindings;
use crate::keyboard_shortcuts::KeyboardShortcutsState;
use crate::language_server_settings::LanguageServerSettingsState;
use crate::pane_group::{PaneGroup, PaneSplitDirection};
use crate::pane_host::{PaneHost, PaneHostScope};
use crate::pane_input::{PaneBinding, PaneInput};
use crate::remote_connection_manager::RemoteConnectionManagerState;
use crate::remote_connection_picker::RemoteConnectionPickerState;
use crate::remote_tunnel_manager::RemoteTunnelManagerState;
use crate::session::session_context_menu::SessionContextMenuState;
use crate::session::session_search::SessionSearch;
use crate::session::session_sidebar::SessionSidebarState;
use crate::shell_interaction::{
    ACTIVE_SESSION_TAB, ADD_SESSION, AGENT_CHANGES, AGENT_EDITOR_PANE, AGENT_EXPLORER_PANE,
    AGENT_FILES, AGENT_FILES_REFRESH, AGENT_FILES_SEARCH, AGENT_SIDEBAR, AGENT_SIDEBAR_NAVIGATION,
    AGENT_SIDEBAR_RESIZE_HANDLE, AGENT_SIDEBAR_TOOLBAR, COMPOSER, COMPOSER_INFO_BAR,
    COMPOSER_PANEL, ContextAction, FILE_EDITOR_DOCUMENT, FILE_EDITOR_PANE, FILE_EDITOR_TAB_LIST,
    MULTI_DIFF_EDITOR, SESSION_CONTEXT_MENU, SESSION_HEADER, SESSION_SEARCH_INPUT,
    SESSION_SIDEBAR_RESIZE_HANDLE, SETTINGS_WORKBENCH_TAB, THREAD_TIMELINE, TITLEBAR,
};
use crate::sidebar_pane_workspace::SidebarPaneWorkspace;
use crate::sidebar_part::SidebarPartState;
use crate::tab_input::TabInput;
use crate::tab_input::TabInputKey;
use crate::thread_projection::ThreadProjection;
use crate::workspace_context::WorkspaceContext;
use crate::workspace_path_picker::WorkspacePathPickerState;
use crate::workspace_surface::WorkspaceSurfaceKind;
use zeta_app_server_protocol::protocol::fs::{FsFileType, FsReadDirectoryEntry};
use zeta_composer::Composer;
use zeta_editor::CodeEditorStyle;
use zeta_settings::SettingsPageSection;
use zeta_terminal::{GridSize, ScreenBuffer, TerminalCore};
use zeta_text_file::{TextFileAccess, TextFileDiskVersion, TextFileModifiedAt, TextFileSnapshot};
use zeta_ui::{
    CaretVisibility, Color, Edges, Point, Rect, ScrollbarPresentation, TextInputCommand,
    TextInputLayoutEngine, UiScene,
};
use zui::runtime::AccessibilityNode;
use zui::ui::{AccessibilityRole, CursorFeedback, DispatchInvalidation, UiDispatch, UiIntent};
use zui::window::WindowControlInsets;

fn viewport() -> LogicalViewport {
    LogicalViewport {
        width: 1000.0,
        height: 700.0,
    }
}

#[test]
fn sidebar_part_outer_border_is_owned_by_native_shell() {
    let bounds = Rect::from_xywh(680.0, 40.0, 320.0, 660.0);
    let mut scene = UiScene::new(crate::shell_style::SHELL_PALETTE.background);

    super::draw_sidebar_part_border(&mut scene, bounds, crate::shell_style::SHELL_PALETTE);

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
    presentation_with_dispatch(terminal, scroll_offset).0
}

fn presentation_with_dispatch(
    terminal: Option<&TerminalCore>,
    scroll_offset: usize,
) -> (ShellPresentation, UiDispatch) {
    let sidebar_pane_workspace = SidebarPaneWorkspace::default();
    let mut dispatch = UiDispatch::default();
    let presentation = presentation_with_workspace(
        terminal,
        scroll_offset,
        SessionSidebarState::collapsed(),
        SidebarPartState::default(),
        SessionContextMenuState::default(),
        &sidebar_pane_workspace,
        &mut dispatch,
    );
    (presentation, dispatch)
}

fn accessibility_nodes(
    presentation: &ShellPresentation,
    dispatch: &UiDispatch,
) -> Vec<AccessibilityNode> {
    presentation
        .interaction_frame()
        .accessibility_nodes(dispatch)
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
        SidebarPartState::default(),
        session_context_menu,
    )
}

fn presentation_with_sidebars_and_menu(
    terminal: Option<&TerminalCore>,
    scroll_offset: usize,
    session_sidebar: SessionSidebarState,
    sidebar_part: SidebarPartState,
    session_context_menu: SessionContextMenuState,
) -> ShellPresentation {
    let sidebar_pane_workspace = SidebarPaneWorkspace::default();
    let mut dispatch = UiDispatch::default();
    presentation_with_workspace(
        terminal,
        scroll_offset,
        session_sidebar,
        sidebar_part,
        session_context_menu,
        &sidebar_pane_workspace,
        &mut dispatch,
    )
}

fn presentation_with_workspace(
    terminal: Option<&TerminalCore>,
    scroll_offset: usize,
    session_sidebar: SessionSidebarState,
    sidebar_part: SidebarPartState,
    session_context_menu: SessionContextMenuState,
    sidebar_pane_workspace: &SidebarPaneWorkspace,
    dispatch: &mut UiDispatch,
) -> ShellPresentation {
    presentation_with_active_tab_input(
        terminal,
        scroll_offset,
        session_sidebar,
        sidebar_part,
        session_context_menu,
        sidebar_pane_workspace,
        dispatch,
        None,
    )
}

fn presentation_with_active_tab_input(
    terminal: Option<&TerminalCore>,
    scroll_offset: usize,
    session_sidebar: SessionSidebarState,
    sidebar_part: SidebarPartState,
    session_context_menu: SessionContextMenuState,
    sidebar_pane_workspace: &SidebarPaneWorkspace,
    dispatch: &mut UiDispatch,
    active_tab_input: Option<TabInputKey>,
) -> ShellPresentation {
    let composer = Composer::default();
    let session_search = SessionSearch::default();
    let workspace_context = WorkspaceContext::fixture("~/Desktop/zeta", Some("main"), Some(0));
    let mut text_layout = TextInputLayoutEngine::new();
    let file_editor_host = FileEditorHost::default();
    let code_editor_style = CodeEditorStyle::light();
    let thread_projection = ThreadProjection::default();
    let workspace_tab_key = TabInputKey::session(
        zeta_protocol::SessionId::new("workspace-pane-session")
            .expect("test session ID is non-empty"),
    );
    let workspace_pane_enabled = sidebar_part.is_expanded() && active_tab_input.is_none();
    let sidebar_part = workspace_pane_enabled
        .then(SidebarPartState::default)
        .unwrap_or(sidebar_part);
    let workspace_pane_group = PaneGroup::new();
    let mut pane_host = PaneHost::new();
    if workspace_pane_enabled {
        pane_host.insert(
            (
                PaneHostScope::Tab(workspace_tab_key.clone()),
                workspace_pane_group.root_pane(),
            ),
            PaneBinding::new(PaneInput::files(
                workspace_context.working_directory().to_path_buf(),
            )),
        );
    }
    let workspace_pane = workspace_pane_enabled.then(|| {
        pane_host
            .mount(
                &PaneHostScope::Tab(workspace_tab_key.clone()),
                &workspace_pane_group,
                workspace_pane_group.root_pane(),
            )
            .expect("workspace pane should mount")
    });
    let active_tab_input = active_tab_input
        .as_ref()
        .or(workspace_pane_enabled.then_some(&workspace_tab_key));
    let pane_group = workspace_pane_enabled.then_some(&workspace_pane_group);
    let tab_inputs = active_tab_input
        .is_some_and(|input| input.is_settings())
        .then(|| vec![TabInput::from_settings()])
        .unwrap_or_default();
    let initial = build_shell_presentation(
        viewport(),
        ShellPresentationModel {
            palette: crate::shell_style::SHELL_PALETTE,
            terminal,
            terminal_panes: &[],
            pane_group,
            active_pane: workspace_pane,
            terminal_pane_resize_split: None,
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
            session_search: &session_search,
            tab_inputs: &tab_inputs,
            active_tab_input,
            caret_visibility: CaretVisibility::Visible,
            dispatch,
            session_sidebar,
            sidebar_part,
            sidebar_pane_workspace,
            session_context_menu,
            git_branch_context_menu: &GitBranchContextMenuState::default(),
            workspace_path_picker: &WorkspacePathPickerState::default(),
            remote_connection_picker: &RemoteConnectionPickerState::default(),
            remote_connection_manager: &RemoteConnectionManagerState::default(),
            remote_tunnel_manager: &RemoteTunnelManagerState::default(),
            keybindings: &NativeKeybindings::default(),
            keyboard_shortcuts: &KeyboardShortcutsState::default(),
            language_server_settings: &LanguageServerSettingsState::default(),
            settings_section: SettingsPageSection::default(),
            language_server_runtime_state: None,
            keybinding_diagnostics: &[],
            theme_scheme: zeta_theme::ColorScheme::Light,
            theme_follows_system: true,
            window_control_insets: WindowControlInsets::NONE,
            pointer_position: None,
        },
        &mut text_layout,
    );
    dispatch.reconcile_focus(&initial.interaction_frame(), COMPOSER);
    build_shell_presentation(
        viewport(),
        ShellPresentationModel {
            palette: crate::shell_style::SHELL_PALETTE,
            terminal,
            terminal_panes: &[],
            pane_group,
            active_pane: workspace_pane,
            terminal_pane_resize_split: None,
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
            session_search: &session_search,
            tab_inputs: &tab_inputs,
            active_tab_input,
            caret_visibility: CaretVisibility::Visible,
            dispatch,
            session_sidebar,
            sidebar_part,
            sidebar_pane_workspace,
            session_context_menu,
            git_branch_context_menu: &GitBranchContextMenuState::default(),
            workspace_path_picker: &WorkspacePathPickerState::default(),
            remote_connection_picker: &RemoteConnectionPickerState::default(),
            remote_connection_manager: &RemoteConnectionManagerState::default(),
            remote_tunnel_manager: &RemoteTunnelManagerState::default(),
            keybindings: &NativeKeybindings::default(),
            keyboard_shortcuts: &KeyboardShortcutsState::default(),
            language_server_settings: &LanguageServerSettingsState::default(),
            settings_section: SettingsPageSection::default(),
            language_server_runtime_state: None,
            keybinding_diagnostics: &[],
            theme_scheme: zeta_theme::ColorScheme::Light,
            theme_follows_system: true,
            window_control_insets: WindowControlInsets::NONE,
            pointer_position: None,
        },
        &mut text_layout,
    )
}

#[test]
fn settings_tab_input_renders_settings_and_selects_the_sidebar_entry() {
    let sidebar_pane_workspace = SidebarPaneWorkspace::default();
    let mut dispatch = UiDispatch::default();
    let presentation = presentation_with_active_tab_input(
        None,
        0,
        SessionSidebarState::expanded(),
        SidebarPartState::default(),
        SessionContextMenuState::default(),
        &sidebar_pane_workspace,
        &mut dispatch,
        Some(TabInputKey::Settings),
    );
    let accessibility_nodes = accessibility_nodes(&presentation, &dispatch);

    assert!(
        presentation
            .scene()
            .text_blocks()
            .iter()
            .any(|text| text.text() == "Settings")
    );
    assert!(
        presentation
            .scene()
            .icons()
            .iter()
            .any(|icon| icon.icon() == zeta_icons::icons::GEAR)
    );
    let node = accessibility_nodes
        .iter()
        .find(|node| node.id == SETTINGS_WORKBENCH_TAB)
        .expect("settings workbench item should be mounted");
    assert_eq!(node.selection, zui::ui::AccessibilitySelection::Selected);
}

#[test]
fn expanded_sidebar_part_file_row_hover_rebuilds_with_the_hover_background() {
    let mut workspace = SidebarPaneWorkspace::default();
    workspace.refresh_files(vec![
        FsReadDirectoryEntry {
            name: "alpha.txt".into(),
            file_type: FsFileType::File,
        },
        FsReadDirectoryEntry {
            name: "beta.txt".into(),
            file_type: FsFileType::File,
        },
    ]);
    let mut dispatch = UiDispatch::default();
    let initial = presentation_with_workspace(
        None,
        0,
        SessionSidebarState::collapsed(),
        SidebarPartState::expanded(),
        SessionContextMenuState::default(),
        &workspace,
        &mut dispatch,
    );
    let accessibility_nodes = accessibility_nodes(&initial, &dispatch);
    let (row_id, row_bounds) = {
        let row = accessibility_nodes
            .iter()
            .find(|node| node.label == "beta.txt")
            .expect("file row should be registered in the shell frame");
        (row.id, row.bounds)
    };
    let point = Point::new(
        row_bounds.origin.x + row_bounds.size.width * 0.5,
        row_bounds.origin.y + row_bounds.size.height * 0.5,
    );

    assert_eq!(initial.interaction_frame().target_at(point), Some(row_id));
    let outcome = dispatch.pointer_moved(point, &initial.interaction_frame());

    assert_eq!(outcome.invalidation, DispatchInvalidation::Paint);
    assert!(dispatch.is_hovered(row_id));
    let hovered = presentation_with_workspace(
        None,
        0,
        SessionSidebarState::collapsed(),
        SidebarPartState::expanded(),
        SessionContextMenuState::default(),
        &workspace,
        &mut dispatch,
    );

    assert!(
        hovered.scene().rects().iter().any(|rect| {
            rect.bounds() == row_bounds && rect.fill() == Color::rgb(242, 242, 242)
        })
    );
}

#[test]
fn primary_layout_keeps_output_above_a_bottom_composer() {
    let layout = ShellLayout::for_viewport(
        viewport(),
        SessionSidebarState::collapsed(),
        SidebarPartState::default(),
    )
    .unwrap();

    assert_eq!(layout.titlebar().origin.y, 0.0);
    assert_eq!(layout.titlebar().size.height, 32.0);
    assert_eq!(layout.main().origin.x, 0.0);
    assert_eq!(layout.main().bottom(), 700.0);
    assert_eq!(layout.output.bottom(), layout.composer_panel.origin.y);
    assert_eq!(layout.composer_panel.origin.y, 572.0);
    assert_eq!(layout.composer_info_bar.origin.y, 580.0);
    assert_eq!(layout.composer.origin.y, 612.0);
    assert_eq!(layout.composer.bottom(), 656.0);
    assert_eq!(layout.composer_toolbar.origin.y, 664.0);
}

#[test]
fn editor_surface_mounts_the_active_file_beside_the_session_canvas() {
    let composer = Composer::default();
    let session_search = SessionSearch::default();
    let workspace_context = WorkspaceContext::fixture("~/Desktop/zeta", Some("main"), Some(0));
    let sidebar_pane_workspace = SidebarPaneWorkspace::default();
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
            terminal_panes: &[],
            pane_group: None,
            active_pane: None,
            terminal_pane_resize_split: None,
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
            session_search: &session_search,
            tab_inputs: &[],
            active_tab_input: None,
            caret_visibility: CaretVisibility::Visible,
            dispatch: &dispatch,
            session_sidebar: SessionSidebarState::collapsed(),
            sidebar_part: SidebarPartState::expanded(),
            sidebar_pane_workspace: &sidebar_pane_workspace,
            session_context_menu: SessionContextMenuState::default(),
            git_branch_context_menu: &GitBranchContextMenuState::default(),
            workspace_path_picker: &WorkspacePathPickerState::default(),
            remote_connection_picker: &RemoteConnectionPickerState::default(),
            remote_connection_manager: &RemoteConnectionManagerState::default(),
            remote_tunnel_manager: &RemoteTunnelManagerState::default(),
            keybindings: &NativeKeybindings::default(),
            keyboard_shortcuts: &KeyboardShortcutsState::default(),
            language_server_settings: &LanguageServerSettingsState::default(),
            settings_section: SettingsPageSection::default(),
            language_server_runtime_state: None,
            keybinding_diagnostics: &[],
            theme_scheme: zeta_theme::ColorScheme::Light,
            theme_follows_system: true,
            window_control_insets: WindowControlInsets::NONE,
            pointer_position: None,
        },
        &mut text_layout,
    );
    let accessibility_nodes = accessibility_nodes(&presentation, &dispatch);

    for id in [FILE_EDITOR_PANE, FILE_EDITOR_TAB_LIST, FILE_EDITOR_DOCUMENT] {
        assert!(accessibility_nodes.iter().any(|node| node.id == id));
    }
    assert!(
        accessibility_nodes
            .iter()
            .any(|node| node.id == FILE_EDITOR_PANE && node.parent == Some(AGENT_SIDEBAR))
    );
    for id in [SESSION_HEADER, COMPOSER, THREAD_TIMELINE] {
        assert!(accessibility_nodes.iter().any(|node| node.id == id));
    }
    assert!(
        presentation
            .scene()
            .text_blocks()
            .iter()
            .any(|block| block.text() == "New session")
    );
    assert!(
        presentation
            .scene()
            .text_blocks()
            .iter()
            .any(|block| block.text() == "fn main() {}")
    );
}

#[test]
fn multiline_composer_grows_upward_between_info_bar_and_bottom_toolbar() {
    let layout = ShellLayout::for_viewport_with_composer_height(
        viewport(),
        SessionSidebarState::collapsed(),
        SidebarPartState::default(),
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
        SessionSidebarState::collapsed(),
        SidebarPartState::default(),
    )
    .unwrap();
    let presentation = presentation(None, 0);
    let composer_panel = presentation
        .scene()
        .rects()
        .iter()
        .find(|rect| rect.bounds() == layout.composer_panel)
        .unwrap();
    let info_editor_separator = presentation
        .scene()
        .rects()
        .iter()
        .find(|rect| rect.bounds() == layout.composer_panel_layout.info_editor_separator())
        .unwrap();

    assert_eq!(presentation.scene().background(), Color::rgb(252, 252, 253));
    assert_eq!(composer_panel.fill(), Color::WHITE);
    assert_eq!(composer_panel.border().widths().top, 1.0);
    assert_eq!(
        info_editor_separator.fill(),
        crate::shell_style::SHELL_PALETTE.border
    );
    let intentional_pills = presentation
        .scene()
        .rects()
        .iter()
        .filter(|rect| rect.corner_radii().top_left > 4.0)
        .collect::<Vec<_>>();
    assert_eq!(intentional_pills.len(), 1);
    assert_eq!(intentional_pills[0].bounds().size.height, 24.0);
}

#[test]
fn primary_presentation_has_an_agent_timeline_and_fixed_composer() {
    let presentation = presentation(None, 0);
    let visible_text = presentation
        .scene()
        .text_blocks()
        .iter()
        .map(|block| block.text())
        .collect::<Vec<_>>();

    assert!(!visible_text.contains(&"zeterm"));
    assert!(!visible_text.contains(&"Starting shell…"));
    assert!(visible_text.contains(&"Ask Zeta anything…"));
    assert!(visible_text.contains(&"Local"));
    assert!(!visible_text.contains(&"Agent"));
    assert!(!visible_text.contains(&"SESSIONS"));
    assert_eq!(presentation.scene().icons().len(), 7);
}

#[test]
fn expanded_sidebar_reflows_the_terminal_and_publishes_a_selected_session_tab() {
    let layout = ShellLayout::for_viewport(
        viewport(),
        SessionSidebarState::expanded(),
        SidebarPartState::default(),
    )
    .unwrap();
    let presentation = presentation_with_sidebar(None, 0, SessionSidebarState::expanded());
    let accessibility_nodes = accessibility_nodes(&presentation, &UiDispatch::default());
    let visible_text = presentation
        .scene()
        .text_blocks()
        .iter()
        .map(|block| block.text())
        .collect::<Vec<_>>();
    let session_tab = accessibility_nodes
        .iter()
        .find(|node| node.id == crate::shell_interaction::ACTIVE_SESSION_TAB)
        .unwrap();
    let resize_handle = accessibility_nodes
        .iter()
        .find(|node| node.id == SESSION_SIDEBAR_RESIZE_HANDLE)
        .unwrap();
    let search = accessibility_nodes
        .iter()
        .find(|node| node.id == SESSION_SEARCH_INPUT)
        .unwrap();
    let add_session = accessibility_nodes
        .iter()
        .find(|node| node.id == ADD_SESSION)
        .unwrap();
    let mut dispatch = UiDispatch::default();

    assert_eq!(layout.session_sidebar().unwrap().size.width, 200.0);
    assert_eq!(layout.main().origin.x, 200.0);
    assert_eq!(layout.composer.origin.x, 224.0);
    assert!(visible_text.contains(&"Search sessions..."));
    let inspected_search = presentation
        .scene()
        .inspection()
        .target_at(Point::new(20.0, 50.0))
        .expect("session search should expose its inspection hierarchy");
    assert_eq!(
        presentation
            .scene()
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
    assert_eq!(
        session_tab.selection,
        zui::ui::AccessibilitySelection::Selected
    );
    assert_eq!(resize_handle.role, AccessibilityRole::Separator);
    assert_eq!(resize_handle.label, "Resize sessions sidebar");
    assert_eq!(resize_handle.value.as_deref(), Some("200 pixels"));
    assert_eq!(
        presentation
            .interaction_frame()
            .target_at(Point::new(200.0, 100.0)),
        Some(SESSION_SIDEBAR_RESIZE_HANDLE)
    );
    dispatch.pointer_moved(Point::new(200.0, 100.0), &presentation.interaction_frame());
    assert_eq!(
        dispatch.pointer_feedback(&presentation.interaction_frame()),
        CursorFeedback::ResizeHorizontal
    );
    assert_eq!(
        terminal_grid_size_for_viewport(
            viewport(),
            ScreenBuffer::Primary,
            SessionSidebarState::expanded(),
            SidebarPartState::default(),
        )
        .cols(),
        94
    );
    assert_eq!(
        terminal_mouse_position_for_viewport(
            viewport(),
            ScreenBuffer::Primary,
            SessionSidebarState::expanded(),
            SidebarPartState::default(),
            Point::new(100.0, 100.0),
        ),
        None
    );
}

#[test]
fn session_search_filters_tabs_by_session_name() {
    let composer = Composer::default();
    let mut session_search = SessionSearch::default();
    session_search.apply(TextInputCommand::Insert("missing session".to_owned()));
    let workspace_context = WorkspaceContext::fixture("~/Desktop/zeta", Some("main"), Some(0));
    let mut text_layout = TextInputLayoutEngine::new();
    let dispatch = UiDispatch::default();
    let sidebar_pane_workspace = SidebarPaneWorkspace::default();
    let file_editor_host = FileEditorHost::default();
    let code_editor_style = CodeEditorStyle::light();
    let thread_projection = ThreadProjection::default();

    let presentation = build_shell_presentation(
        viewport(),
        ShellPresentationModel {
            palette: crate::shell_style::SHELL_PALETTE,
            terminal: None,
            terminal_panes: &[],
            pane_group: None,
            active_pane: None,
            terminal_pane_resize_split: None,
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
            session_search: &session_search,
            tab_inputs: &[],
            active_tab_input: None,
            caret_visibility: CaretVisibility::Visible,
            dispatch: &dispatch,
            session_sidebar: SessionSidebarState::expanded(),
            sidebar_part: SidebarPartState::default(),
            sidebar_pane_workspace: &sidebar_pane_workspace,
            session_context_menu: SessionContextMenuState::default(),
            git_branch_context_menu: &GitBranchContextMenuState::default(),
            workspace_path_picker: &WorkspacePathPickerState::default(),
            remote_connection_picker: &RemoteConnectionPickerState::default(),
            remote_connection_manager: &RemoteConnectionManagerState::default(),
            remote_tunnel_manager: &RemoteTunnelManagerState::default(),
            keybindings: &NativeKeybindings::default(),
            keyboard_shortcuts: &KeyboardShortcutsState::default(),
            language_server_settings: &LanguageServerSettingsState::default(),
            settings_section: SettingsPageSection::default(),
            language_server_runtime_state: None,
            keybinding_diagnostics: &[],
            theme_scheme: zeta_theme::ColorScheme::Light,
            theme_follows_system: true,
            window_control_insets: WindowControlInsets::NONE,
            pointer_position: None,
        },
        &mut text_layout,
    );
    let accessibility_nodes = accessibility_nodes(&presentation, &dispatch);

    assert!(
        accessibility_nodes
            .iter()
            .all(|node| node.id != crate::shell_interaction::ACTIVE_SESSION_TAB)
    );
    assert!(
        presentation
            .scene()
            .text_blocks()
            .iter()
            .any(|block| block.text() == "missing session")
    );
}

#[test]
fn workspace_pane_defaults_to_files_in_the_main_workbench_with_navigation_and_actions() {
    let sidebar_part = SidebarPartState::expanded();
    let presentation = presentation_with_sidebars_and_menu(
        None,
        0,
        SessionSidebarState::collapsed(),
        sidebar_part,
        SessionContextMenuState::default(),
    );
    let accessibility_nodes = accessibility_nodes(&presentation, &UiDispatch::default());
    let sidebar = accessibility_nodes
        .iter()
        .find(|node| node.id == AGENT_SIDEBAR)
        .unwrap();
    let explorer = accessibility_nodes
        .iter()
        .find(|node| node.id == AGENT_EXPLORER_PANE)
        .unwrap();
    let navigation = accessibility_nodes
        .iter()
        .find(|node| node.id == AGENT_SIDEBAR_NAVIGATION)
        .unwrap();
    let toolbar = accessibility_nodes
        .iter()
        .find(|node| node.id == AGENT_SIDEBAR_TOOLBAR)
        .unwrap();
    let resize_handle = accessibility_nodes
        .iter()
        .find(|node| node.id == AGENT_SIDEBAR_RESIZE_HANDLE);

    assert_eq!(
        accessibility_nodes
            .iter()
            .find(|node| node.id == AGENT_SIDEBAR)
            .map(|node| node.bounds),
        Some(zeta_ui::Rect::from_xywh(0.0, 32.0, 1000.0, 668.0))
    );
    assert_eq!(sidebar.role, AccessibilityRole::Group);
    assert_eq!(sidebar.label, "Workspace pane");
    assert_eq!(explorer.parent, Some(AGENT_SIDEBAR));
    assert_eq!(explorer.label, "Files");
    assert_eq!(navigation.role, AccessibilityRole::Toolbar);
    assert_eq!(toolbar.label, "Workspace pane toolbar");
    assert!(resize_handle.is_none());
    assert_eq!(
        toolbar.bounds,
        zeta_ui::Rect::from_xywh(0.0, 32.0, 1000.0, 36.0)
    );
    assert_eq!(
        navigation.bounds,
        zeta_ui::Rect::from_xywh(0.0, 32.0, 128.0, 36.0)
    );
    assert_eq!(navigation.parent, Some(AGENT_SIDEBAR_TOOLBAR));
    assert_eq!(
        explorer.bounds,
        zeta_ui::Rect::from_xywh(0.0, 68.0, 1000.0, 632.0)
    );
    for id in [
        AGENT_CHANGES,
        AGENT_FILES,
        AGENT_FILES_REFRESH,
        AGENT_FILES_SEARCH,
    ] {
        assert!(accessibility_nodes.iter().any(|node| node.id == id));
    }
    assert!(
        accessibility_nodes
            .iter()
            .all(|node| !matches!(node.id, AGENT_EDITOR_PANE | MULTI_DIFF_EDITOR))
    );
    let visible_text = presentation
        .scene()
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
        accessibility_nodes
            .iter()
            .filter(|node| node.parent == Some(crate::shell_interaction::AGENT_FILES_ACTION_BAR))
            .count(),
        2
    );
    assert_eq!(
        terminal_grid_size_for_viewport(
            viewport(),
            ScreenBuffer::Primary,
            SessionSidebarState::collapsed(),
            SidebarPartState::default(),
        )
        .cols(),
        119
    );
}

#[test]
fn changes_switch_mounts_workspace_diffs_in_the_multi_diff_editor_without_files_actions() {
    let composer = Composer::default();
    let session_search = SessionSearch::default();
    let workspace_context = WorkspaceContext::fixture("~/Desktop/zeta", Some("main"), Some(2));
    let mut agent_workspace = SidebarPaneWorkspace::default();
    agent_workspace.sync_repository(&workspace_context);
    let tab_key = TabInputKey::session(
        zeta_protocol::SessionId::new("session-1").expect("test session ID is non-empty"),
    );
    let main_pane_group = PaneGroup::new();
    let mut pane_host = PaneHost::new();
    pane_host.insert(
        (
            PaneHostScope::Tab(tab_key.clone()),
            main_pane_group.root_pane(),
        ),
        PaneBinding::new(PaneInput::diff(
            workspace_context.working_directory().to_path_buf(),
        )),
    );
    let main_pane = pane_host.mount(
        &PaneHostScope::Tab(tab_key.clone()),
        &main_pane_group,
        main_pane_group.root_pane(),
    );
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
            terminal_panes: &[],
            pane_group: Some(&main_pane_group),
            active_pane: main_pane,
            terminal_pane_resize_split: None,
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
            session_search: &session_search,
            tab_inputs: &[],
            active_tab_input: Some(&tab_key),
            caret_visibility: CaretVisibility::Visible,
            dispatch: &dispatch,
            session_sidebar: SessionSidebarState::collapsed(),
            sidebar_part: SidebarPartState::default(),
            sidebar_pane_workspace: &agent_workspace,
            session_context_menu: SessionContextMenuState::default(),
            git_branch_context_menu: &GitBranchContextMenuState::default(),
            workspace_path_picker: &WorkspacePathPickerState::default(),
            remote_connection_picker: &RemoteConnectionPickerState::default(),
            remote_connection_manager: &RemoteConnectionManagerState::default(),
            remote_tunnel_manager: &RemoteTunnelManagerState::default(),
            keybindings: &NativeKeybindings::default(),
            keyboard_shortcuts: &KeyboardShortcutsState::default(),
            language_server_settings: &LanguageServerSettingsState::default(),
            settings_section: SettingsPageSection::default(),
            language_server_runtime_state: None,
            keybinding_diagnostics: &[],
            theme_scheme: zeta_theme::ColorScheme::Light,
            theme_follows_system: true,
            window_control_insets: WindowControlInsets::NONE,
            pointer_position: None,
        },
        &mut text_layout,
    );
    let accessibility_nodes = accessibility_nodes(&presentation, &dispatch);

    assert!(
        accessibility_nodes
            .iter()
            .any(|node| node.id == AGENT_EDITOR_PANE)
    );
    assert!(
        accessibility_nodes
            .iter()
            .any(|node| node.id == MULTI_DIFF_EDITOR)
    );
    assert!(accessibility_nodes.iter().all(|node| !matches!(
        node.id,
        AGENT_EXPLORER_PANE | AGENT_FILES_REFRESH | AGENT_FILES_SEARCH
    )));
    let visible_text = presentation
        .scene()
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
    let accessibility_nodes = accessibility_nodes(&presentation, &UiDispatch::default());
    let labels = accessibility_nodes
        .iter()
        .filter(|node| node.parent == Some(crate::shell_interaction::SESSION_CONTEXT_MENU))
        .map(|node| node.label.as_str())
        .collect::<Vec<_>>();
    let first_item = accessibility_nodes
        .iter()
        .find(|node| {
            node.id == crate::shell_interaction::SessionContextMenuAction::Pin.element_id()
        })
        .unwrap();

    assert_eq!(labels, ["Pin", "Close", "Rename", "Fork"]);
    assert_eq!(
        presentation.interaction_frame().target_at(Point::new(
            first_item.bounds.origin.x + 2.0,
            first_item.bounds.origin.y + 2.0
        )),
        Some(crate::shell_interaction::SessionContextMenuAction::Pin.element_id())
    );
    assert!(
        presentation
            .scene()
            .text_blocks()
            .iter()
            .any(|text| text.text() == "Fork")
    );
}

#[test]
fn primary_presentation_publishes_current_control_semantics_and_focus() {
    let (presentation, dispatch) = presentation_with_dispatch(None, 0);
    let accessibility_nodes = accessibility_nodes(&presentation, &dispatch);
    let info_bar = accessibility_nodes
        .iter()
        .find(|node| node.id == COMPOSER_INFO_BAR)
        .unwrap();
    let composer = accessibility_nodes
        .iter()
        .find(|node| node.id == COMPOSER)
        .unwrap();
    let location = accessibility_nodes
        .iter()
        .find(|node| node.id == ContextAction::Location.element_id())
        .unwrap();

    assert_eq!(info_bar.role, AccessibilityRole::Group);
    assert_eq!(info_bar.label, "/ for commands");
    let inspected_info_bar = presentation
        .scene()
        .inspection()
        .target_at(Point::new(
            info_bar.bounds.origin.x + 100.0,
            info_bar.bounds.origin.y + info_bar.bounds.size.height / 2.0,
        ))
        .expect("composer info bar should expose its inspection hierarchy");
    assert_eq!(
        presentation
            .scene()
            .inspection()
            .ancestry(inspected_info_bar.id())
            .iter()
            .map(|node| node.name())
            .collect::<Vec<_>>(),
        vec!["MainSurface", "ComposerPanel", "ComposerInfoBar"]
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
            .element_bounds(action.element_id())
            .expect("context action should be mounted");
        let point = Point::new(
            bounds.origin.x + bounds.size.width / 2.0,
            bounds.origin.y + bounds.size.height / 2.0,
        );
        let mut dispatch = UiDispatch::default();

        dispatch.pointer_moved(point, &presentation.interaction_frame());
        dispatch.press_primary(&presentation.interaction_frame());
        let outcome = dispatch.release_primary(point, &presentation.interaction_frame());

        assert_eq!(
            outcome.intent,
            Some(UiIntent::Activate(action.element_id()))
        );
    }
}

#[test]
fn overlay_rebuild_restores_the_retained_base_scene_and_interactions() {
    let composer = Composer::default();
    let session_search = SessionSearch::default();
    let workspace_context = WorkspaceContext::fixture("~/Desktop/zeta", Some("main"), Some(0));
    let sidebar_pane_workspace = SidebarPaneWorkspace::default();
    let thread_projection = ThreadProjection::default();
    let git_branch_context_menu = GitBranchContextMenuState::default();
    let workspace_path_picker = WorkspacePathPickerState::default();
    let remote_connection_picker = RemoteConnectionPickerState::default();
    let remote_connection_manager = RemoteConnectionManagerState::default();
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
        terminal_panes: &[],
        pane_group: None,
        active_pane: None,
        terminal_pane_resize_split: None,
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
        session_search: &session_search,
        tab_inputs: &[],
        active_tab_input: None,
        caret_visibility: CaretVisibility::Visible,
        dispatch: &dispatch,
        session_sidebar: SessionSidebarState::collapsed(),
        sidebar_part: SidebarPartState::default(),
        sidebar_pane_workspace: &sidebar_pane_workspace,
        session_context_menu: SessionContextMenuState::default(),
        git_branch_context_menu: &git_branch_context_menu,
        workspace_path_picker: &workspace_path_picker,
        remote_connection_picker: &remote_connection_picker,
        remote_connection_manager: &remote_connection_manager,
        remote_tunnel_manager: &RemoteTunnelManagerState::default(),
        keybindings: &keybindings,
        keyboard_shortcuts: &keyboard_shortcuts,
        language_server_settings: &language_server_settings,
        settings_section: SettingsPageSection::default(),
        language_server_runtime_state: None,
        keybinding_diagnostics: &[],
        theme_scheme: zeta_theme::ColorScheme::Light,
        theme_follows_system: true,
        window_control_insets: WindowControlInsets::NONE,
        pointer_position: None,
    };
    let mut presentation = build_shell_presentation(viewport(), closed_model, &mut text_layout);
    let base_scene = presentation.scene().clone();
    let base_interactions = presentation.interaction_frame().clone();
    let base_accessibility = accessibility_nodes(&presentation, &dispatch);
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
            .interaction_frame()
            .node(SESSION_CONTEXT_MENU)
            .is_some()
    );
    assert_ne!(*presentation.scene(), base_scene);

    assert!(rebuild_shell_overlays(
        &mut presentation,
        viewport(),
        closed_model,
        &mut text_layout,
    ));
    assert_eq!(*presentation.scene(), base_scene);
    assert_eq!(*presentation.interaction_frame(), base_interactions);
    assert_eq!(
        accessibility_nodes(&presentation, &dispatch),
        base_accessibility
    );
}

#[test]
fn titlebar_drags_the_window_and_composer_is_a_registered_input_region() {
    let presentation = presentation(None, 0);
    let mut dispatch = UiDispatch::default();

    assert_eq!(
        dispatch
            .pointer_moved(Point::new(500.0, 17.0), &presentation.interaction_frame())
            .invalidation,
        DispatchInvalidation::Paint
    );
    assert_eq!(
        dispatch
            .press_primary(&presentation.interaction_frame())
            .intent,
        Some(UiIntent::StartWindowDrag(TITLEBAR))
    );
    assert_eq!(
        dispatch
            .pointer_moved(Point::new(500.0, 640.0), &presentation.interaction_frame())
            .invalidation,
        DispatchInvalidation::Paint
    );
    assert_eq!(
        presentation
            .interaction_frame()
            .target_at(Point::new(500.0, 640.0)),
        Some(COMPOSER)
    );
    assert_eq!(
        presentation
            .interaction_frame()
            .target_at(Point::new(28.0, 688.0)),
        Some(COMPOSER_PANEL)
    );
    assert_eq!(
        dispatch.pointer_feedback(&presentation.interaction_frame()),
        CursorFeedback::Text
    );
}

#[test]
fn context_toolbar_starts_with_environment_below_the_composer_editor() {
    let presentation = presentation(None, 0);
    let mut dispatch = UiDispatch::default();

    assert_eq!(
        dispatch
            .pointer_moved(Point::new(40.0, 676.0), &presentation.interaction_frame())
            .invalidation,
        DispatchInvalidation::Paint
    );
    assert_eq!(
        presentation
            .interaction_frame()
            .target_at(Point::new(40.0, 676.0)),
        Some(ContextAction::Location.element_id())
    );
    assert_eq!(
        dispatch.pointer_feedback(&presentation.interaction_frame()),
        CursorFeedback::Pointer
    );
    assert_eq!(
        dispatch
            .press_primary(&presentation.interaction_frame())
            .invalidation,
        DispatchInvalidation::Paint
    );
    assert!(dispatch.is_pressed(ContextAction::Location.element_id()));
}

#[test]
fn compact_viewport_uses_bounded_fallback_scene() {
    let composer = Composer::default();
    let session_search = SessionSearch::default();
    let workspace_context = WorkspaceContext::fixture("/tmp/project", None, None);
    let mut text_layout = TextInputLayoutEngine::new();
    let dispatch = UiDispatch::default();
    let sidebar_pane_workspace = SidebarPaneWorkspace::default();
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
            terminal_panes: &[],
            pane_group: None,
            active_pane: None,
            terminal_pane_resize_split: None,
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
            session_search: &session_search,
            tab_inputs: &[],
            active_tab_input: None,
            caret_visibility: CaretVisibility::Visible,
            dispatch: &dispatch,
            session_sidebar: SessionSidebarState::collapsed(),
            sidebar_part: SidebarPartState::default(),
            sidebar_pane_workspace: &sidebar_pane_workspace,
            session_context_menu: SessionContextMenuState::default(),
            git_branch_context_menu: &GitBranchContextMenuState::default(),
            workspace_path_picker: &WorkspacePathPickerState::default(),
            remote_connection_picker: &RemoteConnectionPickerState::default(),
            remote_connection_manager: &RemoteConnectionManagerState::default(),
            remote_tunnel_manager: &RemoteTunnelManagerState::default(),
            keybindings: &NativeKeybindings::default(),
            keyboard_shortcuts: &KeyboardShortcutsState::default(),
            language_server_settings: &LanguageServerSettingsState::default(),
            settings_section: SettingsPageSection::default(),
            language_server_runtime_state: None,
            keybinding_diagnostics: &[],
            theme_scheme: zeta_theme::ColorScheme::Light,
            theme_follows_system: true,
            window_control_insets: WindowControlInsets::NONE,
            pointer_position: None,
        },
        &mut text_layout,
    );

    assert_eq!(presentation.scene().rects().len(), 1);
    assert_eq!(presentation.scene().text_blocks().len(), 1);
    assert_eq!(presentation.scene().text_blocks()[0].text(), "zeterm");
}

#[test]
fn primary_reserves_rows_for_composer_while_alternate_screen_uses_full_height() {
    let primary = terminal_grid_size_for_viewport(
        viewport(),
        ScreenBuffer::Primary,
        SessionSidebarState::collapsed(),
        SidebarPartState::default(),
    );
    let alternate = terminal_grid_size_for_viewport(
        viewport(),
        ScreenBuffer::Alternate,
        SessionSidebarState::collapsed(),
        SidebarPartState::default(),
    );

    assert_eq!(primary, GridSize::new(27, 119));
    assert_eq!(alternate, GridSize::new(34, 119));
}

#[test]
fn primary_pointer_coordinates_are_limited_to_the_output_region() {
    let first = terminal_mouse_position_for_viewport(
        viewport(),
        ScreenBuffer::Primary,
        SessionSidebarState::collapsed(),
        SidebarPartState::default(),
        Point::new(24.0, 60.0),
    )
    .unwrap();
    let composer = terminal_mouse_position_for_viewport(
        viewport(),
        ScreenBuffer::Primary,
        SessionSidebarState::collapsed(),
        SidebarPartState::default(),
        Point::new(40.0, 640.0),
    );

    assert_eq!((first.row(), first.col()), (0, 0));
    assert_eq!(composer, None);
}

#[test]
fn terminal_pane_sash_hit_uses_the_same_grid_geometry_as_the_panes() {
    let mut group = PaneGroup::new();
    group.split_active(PaneSplitDirection::Horizontal);

    let hit = terminal_pane_sash_for_viewport(
        viewport(),
        ScreenBuffer::Alternate,
        SessionSidebarState::collapsed(),
        SidebarPartState::default(),
        &group,
        Point::new(500.0, 300.0),
    )
    .expect("horizontal terminal Pane Sash");

    assert_eq!(hit.1, zui::ui::SplitViewOrientation::Horizontal);
    assert!(hit.2.resize(80.0).previous_size() > hit.2.resize(0.0).previous_size());
}

#[test]
fn primary_terminal_blocks_do_not_override_the_agent_timeline() {
    let mut terminal = TerminalCore::new(GridSize::new(29, 119));
    terminal.process_output(b"$ ");
    terminal.start_command("printf hi");
    terminal.process_output(b"\x1b[32mhi\x1b[0m\r\n");

    let presentation = presentation(Some(&terminal), 0);
    let visible_text = presentation
        .scene()
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
        .scene()
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
        SessionSidebarState::collapsed(),
        SidebarPartState::default(),
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
        .scene()
        .text_blocks()
        .iter()
        .map(|block| block.text())
        .collect::<Vec<_>>();
    assert!(text.contains(&PRODUCT_DISPLAY_NAME));
    assert!(!text.contains(&"project shell"));
}
