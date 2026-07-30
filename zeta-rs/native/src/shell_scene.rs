use zeta_terminal::{GridSize, ScreenBuffer, TerminalColor, TerminalCore, TerminalMousePosition};
use zeta_ui::{
    Border, CaretVisibility, Color, CornerRadii, FontFamily, FontWeight, InputBox, InputBoxState,
    PaintRect, Rect, TextBlock, TextInput, TextInputLayoutEngine, TextStyle, UiScene,
};

use crate::PRODUCT_DISPLAY_NAME;
use crate::shell_interaction::{ShellHitMap, ShellTarget};
use crate::shell_style::{SHELL_PALETTE, ShellPalette};
use crate::terminal_blocks::{TerminalBlockLineKind, project_block_lines};
use crate::terminal_projection::block_view_range;
use crate::terminal_selection::{TerminalSelectionRange, paint_terminal_selection};
use crate::titlebar::{TITLEBAR_HEIGHT, Titlebar};

const TERMINAL_CELL_WIDTH: f32 = 8.0;
const TERMINAL_LINE_HEIGHT: f32 = 18.0;
const TERMINAL_PADDING: f32 = 24.0;
const COMPOSER_HEIGHT: f32 = 54.0;
const COMPOSER_GAP: f32 = 12.0;

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
    main: Rect,
    output: Rect,
    composer: Rect,
}

impl ShellLayout {
    fn for_viewport(viewport: LogicalViewport) -> Option<Self> {
        if viewport.width < 240.0 || viewport.height < 180.0 {
            return None;
        }
        let titlebar = Rect::from_xywh(0.0, 0.0, viewport.width, TITLEBAR_HEIGHT);
        let body_height = viewport.height - titlebar.size.height;
        let main = Rect::from_xywh(0.0, titlebar.bottom(), viewport.width, body_height);
        let composer = Rect::from_xywh(
            TERMINAL_PADDING,
            main.bottom() - TERMINAL_PADDING - COMPOSER_HEIGHT,
            (main.size.width - TERMINAL_PADDING * 2.0).max(1.0),
            COMPOSER_HEIGHT,
        );
        let output = Rect::from_xywh(
            main.origin.x,
            main.origin.y,
            main.size.width,
            (composer.origin.y - main.origin.y - COMPOSER_GAP).max(1.0),
        );
        Some(Self {
            titlebar,
            main,
            output,
            composer,
        })
    }
}

pub(crate) struct ShellPresentation {
    pub(crate) scene: UiScene,
    pub(crate) hit_map: ShellHitMap,
    pub(crate) ime_cursor_area: Option<Rect>,
}

#[derive(Clone, Copy)]
struct TerminalView<'a> {
    core: Option<&'a TerminalCore>,
    scroll_offset: usize,
    selection: Option<TerminalSelectionRange>,
}

pub(crate) fn build_shell_presentation(
    viewport: LogicalViewport,
    terminal: Option<&TerminalCore>,
    terminal_scroll_offset: usize,
    terminal_selection: Option<TerminalSelectionRange>,
    composer: &TextInput,
    text_layout: &mut TextInputLayoutEngine,
    caret_visibility: CaretVisibility,
) -> ShellPresentation {
    let palette = SHELL_PALETTE;
    let mut scene = UiScene::new(palette.background);
    let mut hit_map = ShellHitMap::default();
    let Some(layout) = ShellLayout::for_viewport(viewport) else {
        draw_compact_scene(&mut scene, viewport, palette);
        return ShellPresentation {
            scene,
            hit_map,
            ime_cursor_area: None,
        };
    };

    let title = terminal
        .and_then(TerminalCore::title)
        .unwrap_or(PRODUCT_DISPLAY_NAME);
    let titlebar = Titlebar::new(layout.titlebar, title, palette);
    titlebar.register_hit_regions(&mut hit_map);
    scene.draw_component(&titlebar);
    let ime_cursor_area = draw_main(
        &mut scene,
        &mut hit_map,
        layout,
        TerminalView {
            core: terminal,
            scroll_offset: terminal_scroll_offset,
            selection: terminal_selection,
        },
        composer,
        text_layout,
        caret_visibility,
    );
    ShellPresentation {
        scene,
        hit_map,
        ime_cursor_area,
    }
}

pub(crate) fn terminal_grid_size_for_viewport(
    viewport: LogicalViewport,
    active_screen: ScreenBuffer,
) -> GridSize {
    let Some(layout) = ShellLayout::for_viewport(viewport) else {
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
    point: zeta_ui::Point,
) -> Option<TerminalMousePosition> {
    let layout = ShellLayout::for_viewport(viewport)?;
    let bounds = terminal_content_bounds(layout, active_screen);
    if !bounds.contains(point) {
        return None;
    }
    let row = ((point.y - bounds.origin.y) / TERMINAL_LINE_HEIGHT).floor() as u16;
    let col = ((point.x - bounds.origin.x) / TERMINAL_CELL_WIDTH).floor() as u16;
    let size = terminal_grid_size_for_viewport(viewport, active_screen);
    (row < size.rows() && col < size.cols()).then(|| TerminalMousePosition::new(row, col))
}

fn draw_main(
    scene: &mut UiScene,
    hit_map: &mut ShellHitMap,
    layout: ShellLayout,
    terminal_view: TerminalView<'_>,
    composer: &TextInput,
    text_layout: &mut TextInputLayoutEngine,
    caret_visibility: CaretVisibility,
) -> Option<Rect> {
    let palette = SHELL_PALETTE;
    let active_screen = terminal_view
        .core
        .map(TerminalCore::active_screen)
        .unwrap_or_default();
    scene.draw_rect(PaintRect::new(layout.main, palette.background));
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
                hit_map,
                layout,
                composer,
                text_layout,
                caret_visibility,
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
    hit_map: &mut ShellHitMap,
    layout: ShellLayout,
    composer: &TextInput,
    text_layout: &mut TextInputLayoutEngine,
    caret_visibility: CaretVisibility,
    palette: ShellPalette,
) -> Option<Rect> {
    scene.draw_rect(PaintRect::new(
        Rect::from_xywh(
            layout.main.origin.x,
            layout.output.bottom(),
            layout.main.size.width,
            1.0,
        ),
        palette.border,
    ));
    hit_map.register(layout.composer, ShellTarget::Composer);
    let input_box = InputBox::new(
        layout.composer,
        "Enter a command…",
        InputBoxState::Focused(caret_visibility),
        palette.composer_style(),
        composer,
        text_layout,
    );
    let caret_bounds = input_box.caret_bounds();
    scene.draw_component(&input_box);
    caret_bounds
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
        TerminalColor::Indexed(index) => indexed_terminal_color(index, palette),
        TerminalColor::Rgb(red, green, blue) => Color::rgb(red, green, blue),
    }
}

fn indexed_terminal_color(index: u8, palette: ShellPalette) -> Color {
    match index {
        0 => Color::rgb(35, 39, 46),
        1 => Color::rgb(224, 108, 117),
        2 => Color::rgb(152, 195, 121),
        3 => Color::rgb(229, 192, 123),
        4 => Color::rgb(97, 175, 239),
        5 => Color::rgb(198, 120, 221),
        6 => Color::rgb(86, 182, 194),
        7 => palette.text,
        8 => Color::rgb(92, 99, 112),
        9 => Color::rgb(240, 113, 120),
        10 => Color::rgb(126, 198, 153),
        11 => Color::rgb(224, 175, 104),
        12 => Color::rgb(106, 169, 255),
        13 => Color::rgb(210, 137, 241),
        14 => Color::rgb(91, 192, 222),
        15 => Color::rgb(248, 248, 242),
        _ => palette.text,
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
