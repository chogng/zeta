use zeta_terminal::{GridSize, ScreenBuffer, TerminalColor, TerminalCore, TerminalMousePosition};
use zeta_ui::{
    Border, CaretVisibility, Color, CornerRadii, FontFamily, FontWeight, InputBox, InputBoxState,
    PaintRect, Rect, Sash, SashOrientation, SashState, SashStyle, TextBlock, TextInput,
    TextInputLayoutEngine, TextStyle, UiScene,
};

use crate::PRODUCT_DISPLAY_NAME;
use crate::agent_sidebar::AgentSidebarState;
use crate::agent_sidebar_layout::AgentSidebarLayout;
use crate::agent_sidebar_workspace::AgentSidebarWorkspace;
use crate::editor_pane::EditorPane;
use crate::explorer_pane::ExplorerPane;
use crate::input_context_toolbar::InputContextToolbar;
use crate::session_context_menu::{SessionContextMenu, SessionContextMenuState};
use crate::session_search::SessionSearch;
use crate::session_sidebar::SessionSidebarState;
use crate::session_sidebar_toolbar::SessionSidebarToolbar;
use crate::session_tab_list::{SessionTab, SessionTabList};
use crate::shell_interaction::{
    ACTIVE_SESSION_TAB, AGENT_SIDEBAR, COMPOSER, COMPOSER_PANEL, MAIN_SURFACE,
    SESSION_SEARCH_INPUT, SESSION_SIDEBAR, SESSION_SIDEBAR_RESIZE_HANDLE, TERMINAL_OUTPUT, WINDOW,
};
use crate::shell_style::{SHELL_PALETTE, ShellPalette};
use crate::terminal_blocks::{TerminalBlockLineKind, project_block_lines};
use crate::terminal_projection::block_view_range;
use crate::terminal_selection::{TerminalSelectionRange, paint_terminal_selection};
use crate::terminal_workspace_layout::TerminalWorkspaceLayout;
use crate::titlebar::{TITLEBAR_HEIGHT, Titlebar};
use crate::workspace_context::WorkspaceContext;
use zeta_ui_dispatch::{
    AccessibilityNode, AccessibilityRole, CursorFeedback, FocusBehavior, InteractionFrame,
    UiDispatch, UiNode,
};
use zeta_winit::WindowControlInsets;

const TERMINAL_CELL_WIDTH: f32 = 8.0;
const TERMINAL_LINE_HEIGHT: f32 = 18.0;
const TERMINAL_PADDING: f32 = 24.0;
const COMPOSER_PANEL_HEIGHT: f32 = 112.0;
const COMPOSER_TOOLBAR_HEIGHT: f32 = 24.0;
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
    composer_panel: Rect,
    composer_toolbar: Rect,
    composer: Rect,
}

impl ShellLayout {
    fn for_viewport(
        viewport: LogicalViewport,
        session_sidebar: SessionSidebarState,
        agent_sidebar: AgentSidebarState,
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
        let composer_panel = Rect::from_xywh(
            main.origin.x,
            main.bottom() - COMPOSER_PANEL_HEIGHT,
            main.size.width,
            COMPOSER_PANEL_HEIGHT,
        );
        let composer = Rect::from_xywh(
            main.origin.x + TERMINAL_PADDING,
            composer_panel.origin.y + 12.0,
            (main.size.width - TERMINAL_PADDING * 2.0).max(1.0),
            COMPOSER_HEIGHT,
        );
        let composer_toolbar = Rect::from_xywh(
            main.origin.x + TERMINAL_PADDING,
            composer_panel.origin.y + 68.0,
            (main.size.width - TERMINAL_PADDING * 2.0).max(1.0),
            COMPOSER_TOOLBAR_HEIGHT,
        );
        let output = Rect::from_xywh(
            main.origin.x,
            main.origin.y,
            main.size.width,
            (composer_panel.origin.y - main.origin.y).max(1.0),
        );
        Some(Self {
            titlebar,
            session_sidebar,
            session_sidebar_sash_track,
            agent_sidebar,
            main,
            output,
            composer_panel,
            composer_toolbar,
            composer,
        })
    }
}

pub(crate) struct ShellPresentation {
    pub(crate) scene: UiScene,
    pub(crate) interaction_frame: InteractionFrame,
    pub(crate) accessibility_nodes: Vec<AccessibilityNode>,
    pub(crate) ime_cursor_area: Option<Rect>,
}

#[derive(Clone, Copy)]
struct TerminalView<'a> {
    core: Option<&'a TerminalCore>,
    scroll_offset: usize,
    selection: Option<TerminalSelectionRange>,
}

pub(crate) struct ShellPresentationModel<'a> {
    pub(crate) terminal: Option<&'a TerminalCore>,
    pub(crate) terminal_scroll_offset: usize,
    pub(crate) terminal_selection: Option<TerminalSelectionRange>,
    pub(crate) workspace_context: &'a WorkspaceContext,
    pub(crate) composer: &'a TextInput,
    pub(crate) session_search: &'a SessionSearch,
    pub(crate) caret_visibility: CaretVisibility,
    pub(crate) dispatch: &'a UiDispatch,
    pub(crate) session_sidebar: SessionSidebarState,
    pub(crate) agent_sidebar: AgentSidebarState,
    pub(crate) agent_sidebar_workspace: &'a AgentSidebarWorkspace,
    pub(crate) session_context_menu: SessionContextMenuState,
    pub(crate) window_control_insets: WindowControlInsets,
}

#[derive(Clone, Copy)]
struct ComposerView<'a> {
    context: &'a WorkspaceContext,
    input: &'a TextInput,
    caret_visibility: CaretVisibility,
    dispatch: &'a UiDispatch,
}

#[derive(Clone, Copy)]
struct SessionSidebarView<'a> {
    title: &'a str,
    context: &'a WorkspaceContext,
    search: &'a SessionSearch,
    caret_visibility: CaretVisibility,
    dispatch: &'a UiDispatch,
}

pub(crate) fn build_shell_presentation(
    viewport: LogicalViewport,
    model: ShellPresentationModel<'_>,
    text_layout: &mut TextInputLayoutEngine,
) -> ShellPresentation {
    let palette = SHELL_PALETTE;
    let mut scene = UiScene::new(palette.background);
    let mut interaction_frame = InteractionFrame::default();
    interaction_frame.register(UiNode::new(
        WINDOW,
        Rect::from_xywh(0.0, 0.0, viewport.width, viewport.height),
        AccessibilityRole::Window,
        PRODUCT_DISPLAY_NAME,
    ));
    let Some(layout) =
        ShellLayout::for_viewport(viewport, model.session_sidebar, model.agent_sidebar)
    else {
        draw_compact_scene(&mut scene, viewport, palette);
        return ShellPresentation {
            scene,
            accessibility_nodes: interaction_frame.accessibility_nodes(model.dispatch),
            interaction_frame,
            ime_cursor_area: None,
        };
    };

    let title = model
        .terminal
        .and_then(TerminalCore::title)
        .unwrap_or(PRODUCT_DISPLAY_NAME);
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
    if let Some(bounds) = layout.agent_sidebar {
        draw_agent_sidebar(
            &mut scene,
            &mut interaction_frame,
            bounds,
            model.agent_sidebar_workspace,
            palette,
        );
    }
    let composer_caret = draw_main(
        &mut scene,
        &mut interaction_frame,
        layout,
        TerminalView {
            core: model.terminal,
            scroll_offset: model.terminal_scroll_offset,
            selection: model.terminal_selection,
        },
        ComposerView {
            context: model.workspace_context,
            input: model.composer,
            caret_visibility: model.caret_visibility,
            dispatch: model.dispatch,
        },
        text_layout,
    );
    let ime_cursor_area = if model.dispatch.is_focused(SESSION_SEARCH_INPUT) {
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
    if let Some(context_menu) = SessionContextMenu::new(
        Rect::from_xywh(0.0, 0.0, viewport.width, viewport.height),
        model.session_context_menu,
        palette,
        model.dispatch,
    ) {
        context_menu.register_interactions(&mut interaction_frame);
        scene.draw_component(&context_menu);
    }
    let accessibility_nodes = interaction_frame.accessibility_nodes(model.dispatch);
    ShellPresentation {
        scene,
        interaction_frame,
        accessibility_nodes,
        ime_cursor_area,
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
    workspace: &AgentSidebarWorkspace,
    palette: ShellPalette,
) {
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
    let explorer = ExplorerPane::new(layout.explorer(), palette);
    explorer.register_interactions(interaction_frame);
    scene.draw_component(&explorer);
    let editor = EditorPane::new(layout.editor(), workspace.editor(), palette);
    editor.register_interactions(interaction_frame);
    scene.draw_component(&editor);
}

fn draw_session_sidebar(
    scene: &mut UiScene,
    interaction_frame: &mut InteractionFrame,
    bounds: Rect,
    view: SessionSidebarView<'_>,
    text_layout: &mut TextInputLayoutEngine,
    palette: ShellPalette,
) -> Option<Rect> {
    scene.draw_rect(
        PaintRect::new(bounds, palette.surface_raised).with_border(Border::new(
            zeta_ui::Edges::new(0.0, 1.0, 0.0, 0.0),
            palette.border,
        )),
    );
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
    terminal_view: TerminalView<'_>,
    composer_view: ComposerView<'_>,
    text_layout: &mut TextInputLayoutEngine,
) -> Option<Rect> {
    let palette = SHELL_PALETTE;
    let active_screen = terminal_view
        .core
        .map(TerminalCore::active_screen)
        .unwrap_or_default();
    scene.draw_rect(PaintRect::new(layout.main, palette.background));
    interaction_frame.register(
        UiNode::new(
            MAIN_SURFACE,
            layout.main,
            AccessibilityRole::Group,
            "Terminal workspace",
        )
        .with_parent(WINDOW)
        .with_cursor(CursorFeedback::Text),
    );
    interaction_frame.register(
        UiNode::new(
            TERMINAL_OUTPUT,
            terminal_content_bounds(layout, active_screen),
            AccessibilityRole::Terminal,
            "Terminal output",
        )
        .with_parent(MAIN_SURFACE)
        .with_cursor(CursorFeedback::Text),
    );
    let mut ime_cursor_area = None;
    scene.with_clip(layout.main, |scene| {
        draw_terminal(
            scene,
            layout,
            terminal_view.core,
            active_screen,
            terminal_view.scroll_offset,
            terminal_view.selection,
            palette,
        );
        if active_screen == ScreenBuffer::Primary {
            ime_cursor_area = draw_composer(
                scene,
                interaction_frame,
                layout,
                composer_view,
                text_layout,
                palette,
            );
        }
    });
    if active_screen == ScreenBuffer::Alternate {
        terminal_view.core.and_then(|terminal| {
            terminal_cursor_area(layout, terminal, terminal_view.scroll_offset)
        })
    } else {
        ime_cursor_area
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
    terminal: Option<&TerminalCore>,
    active_screen: ScreenBuffer,
    scroll_offset: usize,
    selection: Option<TerminalSelectionRange>,
    palette: ShellPalette,
) {
    let bounds = terminal_content_bounds(layout, active_screen);
    let Some(terminal) = terminal else {
        draw_terminal_text(scene, "Starting shell…", bounds, palette.text_muted);
        return;
    };
    if active_screen == ScreenBuffer::Alternate {
        draw_grid(scene, terminal, bounds, scroll_offset, palette);
    } else {
        draw_block_list(scene, terminal, bounds, scroll_offset, palette);
    }
    if active_screen == ScreenBuffer::Primary
        && let Some(selection) = selection
    {
        paint_terminal_selection(
            scene,
            bounds,
            terminal.grid().size().cols() as usize,
            selection,
            TERMINAL_CELL_WIDTH,
            TERMINAL_LINE_HEIGHT,
            palette.terminal_selection,
        );
    }
}

fn draw_composer(
    scene: &mut UiScene,
    interaction_frame: &mut InteractionFrame,
    layout: ShellLayout,
    composer_view: ComposerView<'_>,
    text_layout: &mut TextInputLayoutEngine,
    palette: ShellPalette,
) -> Option<Rect> {
    scene.draw_rect(
        PaintRect::new(layout.composer_panel, palette.surface).with_border(Border::new(
            zeta_ui::Edges::new(1.0, 0.0, 0.0, 0.0),
            palette.border,
        )),
    );
    interaction_frame.register(
        UiNode::new(
            COMPOSER_PANEL,
            layout.composer_panel,
            AccessibilityRole::Group,
            "Command composer",
        )
        .with_parent(MAIN_SURFACE),
    );
    interaction_frame.register(
        UiNode::new(
            COMPOSER,
            layout.composer,
            AccessibilityRole::TextInput,
            "Command input",
        )
        .with_parent(COMPOSER_PANEL)
        .with_cursor(CursorFeedback::Text)
        .with_focus(FocusBehavior::TabStop)
        .with_value(composer_view.input.text()),
    );
    let toolbar = InputContextToolbar::new(
        layout.composer_toolbar,
        composer_view.context,
        palette,
        text_layout,
        composer_view.dispatch,
    );
    toolbar.register_interactions(interaction_frame);
    scene.draw_component(&toolbar);
    let input_state = if composer_view.dispatch.is_focused(COMPOSER) {
        InputBoxState::Focused(composer_view.caret_visibility)
    } else if composer_view.dispatch.is_hovered(COMPOSER) {
        InputBoxState::Hovered
    } else {
        InputBoxState::Resting
    };
    let input_box = InputBox::new(
        layout.composer,
        "Enter a command…",
        input_state,
        palette.composer_style(),
        composer_view.input,
        text_layout,
    );
    let caret_bounds = input_box.caret_bounds();
    scene.draw_component(&input_box);
    composer_view
        .dispatch
        .is_focused(COMPOSER)
        .then_some(caret_bounds)
        .flatten()
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
    palette: ShellPalette,
) {
    let lines = project_block_lines(terminal);
    let capacity = terminal_line_capacity(bounds);
    let range = block_view_range(lines.len(), capacity, scroll_offset);
    for (row, line) in lines[range].iter().enumerate() {
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
                bounds.origin.x,
                bounds.origin.y + row as f32 * TERMINAL_LINE_HEIGHT,
                bounds.size.width,
                TERMINAL_LINE_HEIGHT,
            ),
            color,
        );
    }
}

fn terminal_line_capacity(bounds: Rect) -> usize {
    ((bounds.size.height / TERMINAL_LINE_HEIGHT).floor() as usize).max(1)
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
