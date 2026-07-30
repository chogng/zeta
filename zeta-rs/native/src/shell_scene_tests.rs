use super::{
    LogicalViewport, ShellLayout, ShellPresentation, ShellPresentationModel,
    build_shell_presentation, terminal_grid_size_for_viewport,
    terminal_mouse_position_for_viewport,
};
use crate::agent_sidebar::AgentSidebarState;
use crate::agent_sidebar_workspace::AgentSidebarWorkspace;
use crate::session_context_menu::SessionContextMenuState;
use crate::session_search::SessionSearch;
use crate::session_sidebar::SessionSidebarState;
use crate::shell_interaction::{
    ADD_SESSION, AGENT_EDITOR_PANE, AGENT_EXPLORER_PANE, AGENT_SIDEBAR, COMPOSER, COMPOSER_PANEL,
    ContextAction, MULTI_DIFF_EDITOR, SESSION_SEARCH_INPUT, SESSION_SIDEBAR_RESIZE_HANDLE,
    TITLEBAR,
};
use crate::terminal_projection::scroll_limit;
use crate::workspace_context::WorkspaceContext;
use zeta_terminal::{GridSize, ScreenBuffer, TerminalCore};
use zeta_ui::{CaretVisibility, Color, Point, TextInput, TextInputCommand, TextInputLayoutEngine};
use zeta_ui_dispatch::{
    AccessibilityRole, CursorFeedback, DispatchInvalidation, UiDispatch, UiIntent,
};
use zeta_winit::WindowControlInsets;

fn viewport() -> LogicalViewport {
    LogicalViewport {
        width: 1000.0,
        height: 700.0,
    }
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
    let composer = TextInput::new();
    let session_search = SessionSearch::default();
    let workspace_context = WorkspaceContext::fixture("~/Desktop/zeta", Some("main"), Some(0));
    let mut text_layout = TextInputLayoutEngine::new();
    let mut dispatch = UiDispatch::default();
    let agent_sidebar_workspace = AgentSidebarWorkspace::default();
    let initial = build_shell_presentation(
        viewport(),
        ShellPresentationModel {
            terminal,
            terminal_scroll_offset: scroll_offset,
            terminal_selection: None,
            workspace_context: &workspace_context,
            composer: &composer,
            session_search: &session_search,
            caret_visibility: CaretVisibility::Visible,
            dispatch: &dispatch,
            session_sidebar,
            agent_sidebar,
            agent_sidebar_workspace: &agent_sidebar_workspace,
            session_context_menu,
            window_control_insets: WindowControlInsets::NONE,
        },
        &mut text_layout,
    );
    dispatch.reconcile_focus(&initial.interaction_frame, COMPOSER);
    build_shell_presentation(
        viewport(),
        ShellPresentationModel {
            terminal,
            terminal_scroll_offset: scroll_offset,
            terminal_selection: None,
            workspace_context: &workspace_context,
            composer: &composer,
            session_search: &session_search,
            caret_visibility: CaretVisibility::Visible,
            dispatch: &dispatch,
            session_sidebar,
            agent_sidebar,
            agent_sidebar_workspace: &agent_sidebar_workspace,
            session_context_menu,
            window_control_insets: WindowControlInsets::NONE,
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
    assert!(layout.output.bottom() < layout.composer.origin.y);
    assert_eq!(layout.composer_panel.origin.y, 588.0);
    assert_eq!(layout.composer.bottom(), 644.0);
    assert_eq!(layout.composer_toolbar.origin.y, 656.0);
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

    assert_eq!(presentation.scene.background(), Color::rgb(252, 252, 253));
    assert_eq!(composer_panel.fill(), Color::WHITE);
    assert_eq!(composer_panel.border().widths().top, 1.0);
    assert!(
        presentation
            .scene
            .rects()
            .iter()
            .all(|rect| rect.corner_radii().top_left <= 4.0)
    );
}

#[test]
fn primary_presentation_has_block_output_and_a_fixed_command_editor() {
    let presentation = presentation(None, 0);
    let visible_text = presentation
        .scene
        .text_blocks()
        .iter()
        .map(|block| block.text())
        .collect::<Vec<_>>();

    assert!(!visible_text.contains(&"zeterm"));
    assert!(visible_text.contains(&"Starting shell…"));
    assert!(visible_text.contains(&"Enter a command…"));
    assert!(!visible_text.contains(&"SESSIONS"));
    assert_eq!(presentation.scene.icons().len(), 6);
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
    assert_eq!(search.role, AccessibilityRole::TextInput);
    assert_eq!(add_session.role, AccessibilityRole::Button);
    assert_eq!(add_session.label, "Add new session");
    assert_eq!(session_tab.role, AccessibilityRole::Tab);
    assert_eq!(
        session_tab.selection,
        zeta_ui_dispatch::AccessibilitySelection::Selected
    );
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
    let composer = TextInput::new();
    let mut session_search = SessionSearch::default();
    session_search.apply(TextInputCommand::Insert("missing session".to_owned()));
    let workspace_context = WorkspaceContext::fixture("~/Desktop/zeta", Some("main"), Some(0));
    let mut text_layout = TextInputLayoutEngine::new();
    let dispatch = UiDispatch::default();
    let agent_sidebar_workspace = AgentSidebarWorkspace::default();

    let presentation = build_shell_presentation(
        viewport(),
        ShellPresentationModel {
            terminal: None,
            terminal_scroll_offset: 0,
            terminal_selection: None,
            workspace_context: &workspace_context,
            composer: &composer,
            session_search: &session_search,
            caret_visibility: CaretVisibility::Visible,
            dispatch: &dispatch,
            session_sidebar: SessionSidebarState::expanded(),
            agent_sidebar: AgentSidebarState::default(),
            agent_sidebar_workspace: &agent_sidebar_workspace,
            session_context_menu: SessionContextMenuState::default(),
            window_control_insets: WindowControlInsets::NONE,
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
fn expanded_agent_sidebar_hosts_sibling_explorer_and_editor_panes() {
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
    let editor = presentation
        .accessibility_nodes
        .iter()
        .find(|node| node.id == AGENT_EDITOR_PANE)
        .unwrap();
    let multi_diff = presentation
        .accessibility_nodes
        .iter()
        .find(|node| node.id == MULTI_DIFF_EDITOR)
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
    assert_eq!(explorer.label, "Explorer");
    assert_eq!(editor.parent, Some(AGENT_SIDEBAR));
    assert_eq!(editor.label, "Changed files editor");
    assert_eq!(multi_diff.parent, Some(AGENT_EDITOR_PANE));
    assert_eq!(multi_diff.role, AccessibilityRole::Group);
    assert_eq!(multi_diff.label, "Multiple file differences");
    assert_eq!(explorer.bounds.bottom(), editor.bounds.origin.y);
    let visible_text = presentation
        .scene
        .text_blocks()
        .iter()
        .map(|text| text.text())
        .collect::<Vec<_>>();
    assert!(visible_text.contains(&"Explorer"));
    assert!(visible_text.contains(&"No files loaded"));
    assert!(visible_text.contains(&"No changed files"));
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

    assert_eq!(composer.role, AccessibilityRole::TextInput);
    assert_eq!(composer.label, "Command input");
    assert_eq!(composer.value.as_deref(), Some(""));
    assert!(composer.focused);
    assert_eq!(location.role, AccessibilityRole::Button);
    assert_eq!(location.label, "Environment: Local");
    assert!(!location.focused);
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
fn context_toolbar_registers_button_geometry_above_the_composer_panel() {
    let presentation = presentation(None, 0);
    let mut dispatch = UiDispatch::default();

    assert_eq!(
        dispatch
            .pointer_moved(Point::new(40.0, 668.0), &presentation.interaction_frame)
            .invalidation,
        DispatchInvalidation::Paint
    );
    assert_eq!(
        presentation
            .interaction_frame
            .target_at(Point::new(40.0, 668.0)),
        Some(ContextAction::Location.element_id())
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
    assert!(dispatch.is_pressed(ContextAction::Location.element_id()));
}

#[test]
fn compact_viewport_uses_bounded_fallback_scene() {
    let composer = TextInput::new();
    let session_search = SessionSearch::default();
    let workspace_context = WorkspaceContext::fixture("/tmp/project", None, None);
    let mut text_layout = TextInputLayoutEngine::new();
    let dispatch = UiDispatch::default();
    let agent_sidebar_workspace = AgentSidebarWorkspace::default();
    let presentation = build_shell_presentation(
        LogicalViewport {
            width: 220.0,
            height: 100.0,
        },
        ShellPresentationModel {
            terminal: None,
            terminal_scroll_offset: 0,
            terminal_selection: None,
            workspace_context: &workspace_context,
            composer: &composer,
            session_search: &session_search,
            caret_visibility: CaretVisibility::Visible,
            dispatch: &dispatch,
            session_sidebar: SessionSidebarState::default(),
            agent_sidebar: AgentSidebarState::default(),
            agent_sidebar_workspace: &agent_sidebar_workspace,
            session_context_menu: SessionContextMenuState::default(),
            window_control_insets: WindowControlInsets::NONE,
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

    assert_eq!(primary, GridSize::new(28, 119));
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
fn primary_block_list_is_projected_above_the_composer() {
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

    assert!(visible_text.contains(&"❯ printf hi"));
    assert!(visible_text.contains(&"hi"));
    assert!(visible_text.contains(&"Enter a command…"));
}

#[test]
fn primary_block_transcript_can_project_an_older_viewport() {
    let mut terminal = TerminalCore::new(GridSize::new(29, 119));
    terminal.start_command("history");
    for index in 0..80 {
        terminal.process_output(format!("line-{index}\r\n").as_bytes());
    }
    let capacity = terminal_grid_size_for_viewport(
        viewport(),
        ScreenBuffer::Primary,
        SessionSidebarState::default(),
        AgentSidebarState::default(),
    )
    .rows() as usize;
    let limit = scroll_limit(&terminal, capacity);

    let presentation = presentation(Some(&terminal), limit);
    let visible_text = presentation
        .scene
        .text_blocks()
        .iter()
        .map(|block| block.text())
        .collect::<Vec<_>>();

    assert!(limit > 0);
    assert!(visible_text.contains(&"❯ history"));
    assert!(visible_text.contains(&"line-0"));
    assert!(!visible_text.contains(&"line-79"));
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
fn osc_title_is_projected_into_the_expanded_session_tab() {
    let mut terminal = TerminalCore::new(GridSize::new(29, 119));
    terminal.process_output(b"\x1b]2;project shell\x07");

    let presentation =
        presentation_with_sidebar(Some(&terminal), 0, SessionSidebarState::expanded());

    assert!(
        presentation
            .scene
            .text_blocks()
            .iter()
            .any(|block| block.text() == "project shell")
    );
}
