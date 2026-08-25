use crate::ui::Color;
use crate::ui::Point;
use crate::ui::Rect;
use crate::ui::Size;
use crate::ui::TextBlock;
use crate::ui::TextBlockWrap;
use crate::ui::TextStyle;
use crate::ui::UiScene;

pub(crate) fn metrics(node: &crate::ui::InspectionNode) -> String {
    let mut value = format!("size {:.0} × {:.0}", node.width(), node.height());
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

pub(crate) fn source(node: &crate::ui::InspectionNode) -> String {
    if node.source_file().is_empty() || node.source_line() == 0 {
        return "source unavailable".to_owned();
    }
    let file = node
        .source_file()
        .rsplit('/')
        .next()
        .unwrap_or(node.source_file());
    format!("{file}:{}  ·  layer {}", node.source_line(), node.layer())
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
