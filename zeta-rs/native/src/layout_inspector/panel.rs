use zeta_icons::icons;
use zeta_ui::{
    Border, Button, ButtonBackgrounds, ButtonSelection, ButtonState, ButtonStyle, Color,
    CornerRadii, Edges, FontFamily, FontWeight, PaintRect, Point, Rect, Size, TextBlock,
    TextBlockWrap, TextStyle, UiScene,
};

use super::InspectionSelection;
use crate::shell_scene::LogicalViewport;

const PANEL_BACKGROUND: Color = Color::rgb(248, 248, 250);
const PANEL_BORDER: Color = Color::rgb(218, 218, 224);
const ROW_BACKGROUND: Color = Color::rgba(35, 131, 226, 24);
const FOREGROUND: Color = Color::rgb(35, 35, 42);
const MUTED: Color = Color::rgb(105, 105, 116);
const ACCENT: Color = Color::rgb(35, 131, 226);
const PANEL_PADDING: f32 = 16.0;
const HEADER_HEIGHT: f32 = 62.0;
const PICKER_SIZE: f32 = 28.0;
const PICKER_INSET: f32 = 8.0;
const HEADER_TEXT_X: f32 = 48.0;
const ROW_HEIGHT: f32 = 74.0;
const INDENT: f32 = 12.0;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(super) struct PanelState {
    pub(super) picking: bool,
    pub(super) picker_hovered: bool,
    pub(super) has_selection: bool,
}

pub(super) fn picker_bounds(content_width: f32) -> Rect {
    Rect::from_xywh(
        content_width + PICKER_INSET,
        PICKER_INSET,
        PICKER_SIZE,
        PICKER_SIZE,
    )
}

pub(super) fn paint(
    scene: &mut UiScene,
    viewport: LogicalViewport,
    content_width: f32,
    selection: Option<&InspectionSelection>,
    state: PanelState,
) {
    let panel_width = (viewport.width - content_width).max(0.0);
    if panel_width <= 0.0 || viewport.height <= 0.0 {
        return;
    }
    let bounds = Rect::from_xywh(content_width, 0.0, panel_width, viewport.height);
    scene.with_clip(bounds, |scene| {
        scene.draw_rect(
            PaintRect::new(bounds, PANEL_BACKGROUND)
                .with_border(Border::new(Edges::new(0.0, 0.0, 0.0, 1.0), PANEL_BORDER)),
        );
        let picker = Button::icon(
            picker_bounds(content_width),
            if state.picking {
                icons::CURSOR_FILLED
            } else {
                icons::CURSOR
            },
            if state.picking {
                "Stop selecting components"
            } else {
                "Select a component"
            },
            if state.picker_hovered {
                ButtonState::Hovered
            } else {
                ButtonState::Resting
            },
            picker_style(),
        )
        .with_selection(if state.picking {
            ButtonSelection::Selected
        } else {
            ButtonSelection::Unselected
        });
        scene.draw_component(&picker);
        paint_text(
            scene,
            if state.has_selection {
                "Layout Inspector  •  Selected"
            } else {
                "Layout Inspector"
            },
            Point::new(content_width + HEADER_TEXT_X, 14.0),
            panel_width - HEADER_TEXT_X - PANEL_PADDING,
            TextStyle::new(15.0, FOREGROUND)
                .with_weight(FontWeight::Bold)
                .with_line_height(20.0),
        );
        paint_text(
            scene,
            if state.picking {
                "Click an element · Esc to stop selecting"
            } else {
                "Use the cursor tool to select an element"
            },
            Point::new(content_width + HEADER_TEXT_X, 38.0),
            panel_width - HEADER_TEXT_X - PANEL_PADDING,
            TextStyle::new(11.0, MUTED).with_line_height(16.0),
        );
        scene.draw_rect(PaintRect::new(
            Rect::from_xywh(content_width, HEADER_HEIGHT - 1.0, panel_width, 1.0),
            PANEL_BORDER,
        ));

        let Some(selection) = selection else {
            paint_text(
                scene,
                if state.picking {
                    "Move over the application, then click an element to inspect it."
                } else {
                    "Select the cursor tool to start inspecting the application."
                },
                Point::new(content_width + PANEL_PADDING, HEADER_HEIGHT + PANEL_PADDING),
                panel_width - PANEL_PADDING * 2.0,
                TextStyle::new(12.0, MUTED).with_line_height(18.0),
            );
            return;
        };
        for (depth, node) in selection.path.iter().enumerate() {
            let y = HEADER_HEIGHT + depth as f32 * ROW_HEIGHT;
            if y >= viewport.height {
                break;
            }
            let row_bounds = Rect::from_xywh(content_width, y, panel_width, ROW_HEIGHT);
            let selected = depth + 1 == selection.path.len();
            if selected {
                scene.draw_rect(PaintRect::new(row_bounds, ROW_BACKGROUND));
                scene.draw_rect(PaintRect::new(
                    Rect::from_xywh(content_width, y, 3.0, ROW_HEIGHT),
                    ACCENT,
                ));
            }
            paint_row(scene, node, content_width, panel_width, y, depth, selected);
            scene.draw_rect(PaintRect::new(
                Rect::from_xywh(
                    content_width + PANEL_PADDING,
                    y + ROW_HEIGHT - 1.0,
                    panel_width - PANEL_PADDING,
                    1.0,
                ),
                PANEL_BORDER,
            ));
        }
    });
}

fn picker_style() -> ButtonStyle {
    ButtonStyle::new(
        ButtonBackgrounds::new(Color::TRANSPARENT).with_hovered(Color::rgba(35, 131, 226, 24)),
        TextStyle::new(12.0, FOREGROUND),
    )
    .with_selected_backgrounds(ButtonBackgrounds::new(Color::rgba(35, 131, 226, 40)))
    .with_corner_radii(CornerRadii::uniform(4.0))
    .with_padding(Edges::uniform(6.0))
    .with_icon_size(16.0)
}

fn paint_row(
    scene: &mut UiScene,
    node: &zeta_ui::InspectionNode,
    panel_x: f32,
    panel_width: f32,
    y: f32,
    depth: usize,
    selected: bool,
) {
    let x = panel_x + PANEL_PADDING + depth as f32 * INDENT;
    let available_width = (panel_width - (x - panel_x) - PANEL_PADDING).max(0.0);
    let marker = if selected { "◆" } else { "└" };
    paint_text(
        scene,
        &format!("{marker}  {}", node.name()),
        Point::new(x, y + 8.0),
        available_width,
        TextStyle::new(12.0, if selected { ACCENT } else { FOREGROUND })
            .with_weight(FontWeight::Bold)
            .with_line_height(16.0),
    );
    paint_text(
        scene,
        &metrics(node),
        Point::new(x + 18.0, y + 29.0),
        (available_width - 18.0).max(0.0),
        TextStyle::new(11.0, FOREGROUND)
            .with_family(FontFamily::Monospace)
            .with_line_height(15.0),
    );
    paint_text(
        scene,
        &source(node),
        Point::new(x + 18.0, y + 49.0),
        (available_width - 18.0).max(0.0),
        TextStyle::new(10.0, MUTED)
            .with_family(FontFamily::Monospace)
            .with_line_height(14.0),
    );
}

fn metrics(node: &zeta_ui::InspectionNode) -> String {
    let mut value = format!("size {:.0} × {:.0}", node.width(), node.height());
    if let Some(Edges {
        top,
        right,
        bottom,
        left,
    }) = node.padding()
    {
        value.push_str(&format!(
            "   padding {:.0} {:.0} {:.0} {:.0}",
            top, right, bottom, left
        ));
    }
    if let Some(CornerRadii {
        top_left,
        top_right,
        bottom_right,
        bottom_left,
    }) = node.corner_radii()
    {
        if top_left == top_right && top_left == bottom_right && top_left == bottom_left {
            value.push_str(&format!("   radius {top_left:.0}"));
        } else {
            value.push_str(&format!(
                "   radius {:.0} {:.0} {:.0} {:.0}",
                top_left, top_right, bottom_right, bottom_left
            ));
        }
    }
    value
}

fn source(node: &zeta_ui::InspectionNode) -> String {
    let file = node
        .source_file()
        .rsplit('/')
        .next()
        .unwrap_or(node.source_file());
    format!("{file}:{}  ·  layer {}", node.source_line(), node.layer())
}

fn paint_text(scene: &mut UiScene, text: &str, origin: Point, width: f32, style: TextStyle) {
    if width <= 0.0 {
        return;
    }
    scene.draw_text(
        TextBlock::new(text, origin, Size::new(width, style.line_height()), style)
            .with_wrap(TextBlockWrap::None),
    );
}
