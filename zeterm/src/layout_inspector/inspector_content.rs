use zeta_ui::{
    Color, Component, ComponentElement, CornerRadii, Edges, Element, ElementDirection,
    ElementLength, FontFamily, FontWeight, PaintRect, Point, Rect, Size, TextBlock, TextBlockWrap,
    TextStyle, UiScene,
};

use super::InspectionSelection;

const ROW_BACKGROUND: Color = Color::rgba(35, 131, 226, 24);
const ROW_HOVER_BACKGROUND: Color = Color::rgba(35, 35, 42, 10);
const FOREGROUND: Color = Color::rgb(35, 35, 42);
const MUTED: Color = Color::rgb(105, 105, 116);
const ACCENT: Color = Color::rgb(35, 131, 226);
const CONTENT_PADDING: f32 = 16.0;
const ROW_HEIGHT: f32 = 90.0;
const INDENT: f32 = 12.0;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(super) struct InspectorContentState {
    pub(super) picking: bool,
    pub(super) hovered_row: Option<usize>,
}

pub(super) struct InspectorContent<'a> {
    bounds: Rect,
    selection: Option<&'a InspectionSelection>,
    state: InspectorContentState,
}

impl<'a> InspectorContent<'a> {
    pub(super) const fn new(
        bounds: Rect,
        selection: Option<&'a InspectionSelection>,
        state: InspectorContentState,
    ) -> Self {
        Self {
            bounds,
            selection,
            state,
        }
    }

    pub(super) fn row_index_at(bounds: Rect, point: Point, row_count: usize) -> Option<usize> {
        if !bounds.contains(point) {
            return None;
        }
        let index = ((point.y - bounds.origin.y) / ROW_HEIGHT).floor() as usize;
        (index < row_count).then_some(index)
    }
}

impl Component for InspectorContent<'_> {
    fn element(&self) -> ComponentElement {
        Element::leaf("InspectorContent").in_bounds(self.bounds)
    }

    fn paint(&self, scene: &mut UiScene) {
        scene.with_clip(self.bounds, |scene| {
            let Some(selection) = self.selection else {
                paint_text(
                    scene,
                    if self.state.picking {
                        "Move over the application, then click an element to inspect it."
                    } else {
                        "Select the cursor tool to start inspecting the application."
                    },
                    Point::new(
                        self.bounds.origin.x + CONTENT_PADDING,
                        self.bounds.origin.y + CONTENT_PADDING,
                    ),
                    self.bounds.size.width - CONTENT_PADDING * 2.0,
                    TextStyle::new(12.0, MUTED).with_line_height(18.0),
                );
                return;
            };
            for (depth, node) in selection.path.iter().enumerate() {
                let y = self.bounds.origin.y + depth as f32 * ROW_HEIGHT;
                if y >= self.bounds.bottom() {
                    break;
                }
                let row_bounds =
                    Rect::from_xywh(self.bounds.origin.x, y, self.bounds.size.width, ROW_HEIGHT);
                let selected = depth == selection.selected_index();
                if selected {
                    scene.draw_rect(PaintRect::new(row_bounds, ROW_BACKGROUND));
                    scene.draw_rect(PaintRect::new(
                        Rect::from_xywh(self.bounds.origin.x, y, 3.0, ROW_HEIGHT),
                        ACCENT,
                    ));
                } else if self.state.hovered_row == Some(depth) {
                    scene.draw_rect(PaintRect::new(row_bounds, ROW_HOVER_BACKGROUND));
                }
                paint_row(
                    scene,
                    node,
                    self.bounds.origin.x,
                    self.bounds.size.width,
                    y,
                    depth,
                    selected,
                );
                scene.draw_rect(PaintRect::new(
                    Rect::from_xywh(
                        self.bounds.origin.x + CONTENT_PADDING,
                        y + ROW_HEIGHT - 1.0,
                        self.bounds.size.width - CONTENT_PADDING,
                        1.0,
                    ),
                    Color::rgb(218, 218, 224),
                ));
            }
        });
    }
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
    let x = panel_x + CONTENT_PADDING + depth as f32 * INDENT;
    let available_width = (panel_width - (x - panel_x) - CONTENT_PADDING).max(0.0);
    let marker = if selected { "◆" } else { "└" };
    let title = node
        .label()
        .map(|label| format!("{}  {label}", node.name()))
        .unwrap_or_else(|| node.name().to_owned());
    paint_text(
        scene,
        &format!("{marker}  {title}"),
        Point::new(x, y + 8.0),
        available_width,
        TextStyle::new(12.0, if selected { ACCENT } else { FOREGROUND })
            .with_weight(FontWeight::Bold)
            .with_line_height(16.0),
    );
    paint_text(
        scene,
        &metrics(node),
        Point::new(x + 18.0, y + 27.0),
        (available_width - 18.0).max(0.0),
        TextStyle::new(11.0, FOREGROUND)
            .with_family(FontFamily::Monospace)
            .with_line_height(15.0),
    );
    paint_text(
        scene,
        &authored_layout(node),
        Point::new(x + 18.0, y + 46.0),
        (available_width - 18.0).max(0.0),
        TextStyle::new(10.0, MUTED)
            .with_family(FontFamily::Monospace)
            .with_line_height(14.0),
    );
    paint_text(
        scene,
        &source(node),
        Point::new(x + 18.0, y + 65.0),
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
    if let Some(gap) = node.gap() {
        value.push_str(&format!("   gap {gap:.0}"));
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

fn authored_layout(node: &zeta_ui::InspectionNode) -> String {
    let Some(style) = node.authored_style() else {
        return String::new();
    };
    let direction = match style.direction() {
        ElementDirection::Horizontal => "row",
        ElementDirection::Vertical => "column",
    };
    format!(
        "{direction}   width {}   height {}",
        length(style.width()),
        length(style.height())
    )
}

fn length(value: ElementLength) -> String {
    match value {
        ElementLength::Fill => "fill".to_owned(),
        ElementLength::Pixels(value) => format!("{value:.0}"),
    }
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

#[cfg(test)]
#[path = "inspector_content_tests.rs"]
mod tests;
