use crate::ui::Color;
use crate::ui::FontFamily;
use crate::ui::FontWeight;
use crate::ui::InspectionNode;
use crate::ui::PaintRect;
use crate::ui::Point;
use crate::ui::Rect;
use crate::ui::Size;
use crate::ui::TextBlock;
use crate::ui::TextBlockWrap;
use crate::ui::TextStyle;
use crate::ui::UiScene;

use super::computed_contract::COMPUTED_SECTIONS;
use super::computed_contract::computed_values;

const DETAIL_FOREGROUND: Color = Color::rgb(35, 35, 42);
const DETAIL_MUTED: Color = Color::rgb(105, 105, 116);
const DETAIL_BORDER: Color = Color::rgb(218, 218, 224);
const DETAIL_ACCENT: Color = Color::rgb(35, 131, 226);
const DETAIL_PADDING: f32 = 16.0;
const DETAIL_VALUE_OFFSET: f32 = 126.0;

pub(crate) fn metrics(node: &crate::ui::InspectionNode) -> String {
    let bounds = node.bounds();
    let mut value = format!(
        "size {:.0} × {:.0}   position {:.0}, {:.0}",
        bounds.size.width, bounds.size.height, bounds.origin.x, bounds.origin.y
    );
    if let Some(padding) = node.padding() {
        value.push_str(&format!(
            "   padding {:.0} {:.0} {:.0} {:.0}",
            padding.top, padding.right, padding.bottom, padding.left
        ));
    }
    if let Some(gap) = node.gap() {
        value.push_str(&format!("   gap {gap:.0}"));
    }
    if let Some(radii) = node.corner_radii() {
        value.push_str(&format!("   radius {:.0}", radii.top_left));
    }
    value
}

pub(crate) fn paint_computed(scene: &mut UiScene, bounds: Rect, node: Option<&InspectionNode>) {
    if bounds.is_empty() {
        return;
    }
    let content_width = (bounds.size.width - DETAIL_PADDING * 2.0).max(0.0);
    let origin_x = bounds.origin.x + DETAIL_PADDING;
    let mut y = bounds.origin.y + DETAIL_PADDING;
    paint_text(
        scene,
        "Styles",
        Point::new(origin_x, y),
        60.0,
        TextStyle::new(12.0, DETAIL_MUTED)
            .with_weight(FontWeight::Bold)
            .with_line_height(18.0),
    );
    paint_text(
        scene,
        "Computed",
        Point::new(origin_x + 72.0, y),
        (content_width - 72.0).max(0.0),
        TextStyle::new(12.0, DETAIL_ACCENT)
            .with_weight(FontWeight::Bold)
            .with_line_height(18.0),
    );
    scene.draw_rect(PaintRect::new(
        Rect::from_xywh(origin_x + 64.0, y + 28.0, 96.0, 2.0),
        DETAIL_ACCENT,
    ));
    y += 44.0;

    let Some(node) = node else {
        paint_text(
            scene,
            "Select an element to inspect its layout.",
            Point::new(origin_x, y),
            content_width,
            TextStyle::new(11.0, DETAIL_MUTED).with_line_height(16.0),
        );
        return;
    };

    let title = node
        .label()
        .map(|label| format!("{}  {label}", node.name()))
        .unwrap_or_else(|| node.name().to_owned());
    paint_text(
        scene,
        &title,
        Point::new(origin_x, y),
        content_width,
        TextStyle::new(12.0, DETAIL_ACCENT)
            .with_weight(FontWeight::Bold)
            .with_line_height(16.0),
    );
    y += 20.0;
    paint_text(
        scene,
        &metrics(node),
        Point::new(origin_x, y),
        content_width,
        TextStyle::new(10.0, DETAIL_MUTED)
            .with_family(FontFamily::Monospace)
            .with_line_height(14.0),
    );
    y += 24.0;
    paint_rule(scene, bounds, y);
    y += 12.0;

    let values = computed_values(node);
    for section in COMPUTED_SECTIONS {
        paint_section(scene, origin_x, content_width, &mut y, section.label);
        for field in section.fields {
            let value = values
                .iter()
                .find(|value| value.id == field.id)
                .map(|value| value.text.as_str())
                .unwrap_or("—");
            paint_detail_row(scene, bounds, y, field.label, value);
            y += 20.0;
        }
        y += 8.0;
        paint_rule(scene, bounds, y - 8.0);
    }
}

fn paint_section(scene: &mut UiScene, origin_x: f32, width: f32, y: &mut f32, label: &str) {
    paint_text(
        scene,
        label,
        Point::new(origin_x, *y),
        width,
        TextStyle::new(11.0, DETAIL_FOREGROUND)
            .with_weight(FontWeight::Bold)
            .with_line_height(16.0),
    );
    *y += 22.0;
}

fn paint_detail_row(scene: &mut UiScene, bounds: Rect, y: f32, label: &str, value: &str) {
    let origin_x = bounds.origin.x + DETAIL_PADDING;
    paint_text(
        scene,
        label,
        Point::new(origin_x, y),
        DETAIL_VALUE_OFFSET,
        TextStyle::new(10.0, DETAIL_MUTED).with_line_height(16.0),
    );
    paint_text(
        scene,
        value,
        Point::new(origin_x + DETAIL_VALUE_OFFSET, y),
        (bounds.size.width - DETAIL_PADDING * 2.0 - DETAIL_VALUE_OFFSET).max(0.0),
        TextStyle::new(10.0, DETAIL_FOREGROUND)
            .with_family(FontFamily::Monospace)
            .with_line_height(16.0),
    );
}

fn paint_rule(scene: &mut UiScene, bounds: Rect, y: f32) {
    scene.draw_rect(PaintRect::new(
        Rect::from_xywh(bounds.origin.x, y, bounds.size.width, 1.0),
        DETAIL_BORDER,
    ));
}

pub(crate) fn paint_message(
    scene: &mut UiScene,
    message: &str,
    bounds: Rect,
    padding: f32,
    color: Color,
) {
    paint_text(
        scene,
        message,
        Point::new(bounds.origin.x + padding, bounds.origin.y + padding),
        (bounds.size.width - padding * 2.0).max(0.0),
        TextStyle::new(12.0, color).with_line_height(18.0),
    );
}

pub(crate) fn paint_text(
    scene: &mut UiScene,
    text: &str,
    origin: Point,
    width: f32,
    style: TextStyle,
) {
    if width <= 0.0 {
        return;
    }
    scene.draw_text(
        TextBlock::new(text, origin, Size::new(width, style.line_height()), style)
            .with_wrap(TextBlockWrap::None),
    );
}
