use crate::{
    Color, Component, ComponentElement, Element, PaintIcon, Point, Rect, TextBlock, TextStyle,
    UiScene,
};
use zui::ui::{Icon, TextSpan};

/// Presentation metrics and colors for an icon followed by a single text label.
#[derive(Clone, Debug, PartialEq)]
pub struct IconLabelStyle {
    text_style: TextStyle,
    icon_color: Color,
    icon_size: f32,
    content_gap: f32,
}

impl IconLabelStyle {
    pub fn new(text_style: TextStyle) -> Self {
        Self {
            icon_color: text_style.color(),
            text_style,
            icon_size: 16.0,
            content_gap: 4.0,
        }
    }

    pub const fn with_icon_size(mut self, icon_size: f32) -> Self {
        self.icon_size = icon_size;
        self
    }

    /// Overrides the icon tint without changing the label text color.
    pub const fn with_icon_color(mut self, icon_color: Color) -> Self {
        self.icon_color = icon_color;
        self
    }

    pub const fn with_content_gap(mut self, content_gap: f32) -> Self {
        self.content_gap = content_gap;
        self
    }
}

/// Reusable presentation component that aligns one semantic icon with a text label.
#[derive(Clone, Debug, PartialEq)]
pub struct IconLabel {
    bounds: Rect,
    icon: Icon,
    label: String,
    spans: Vec<TextSpan>,
    style: IconLabelStyle,
}

impl IconLabel {
    pub fn new(bounds: Rect, icon: Icon, label: impl Into<String>, style: IconLabelStyle) -> Self {
        Self {
            bounds,
            icon,
            label: label.into(),
            spans: Vec::new(),
            style,
        }
    }

    /// Creates an icon label whose visible text uses multiple colors or font treatments.
    pub fn from_spans(
        bounds: Rect,
        icon: Icon,
        spans: impl IntoIterator<Item = TextSpan>,
        style: IconLabelStyle,
    ) -> Self {
        let spans = spans.into_iter().collect::<Vec<_>>();
        let label = spans.iter().map(TextSpan::text).collect::<String>();
        Self {
            bounds,
            icon,
            label,
            spans,
            style,
        }
    }

    pub const fn bounds(&self) -> Rect {
        self.bounds
    }
}

impl Component for IconLabel {
    fn element(&self) -> ComponentElement {
        Element::leaf("IconLabel")
            .in_bounds(self.bounds)
            .with_inspection_label(&self.label)
    }

    fn paint(&self, scene: &mut UiScene) {
        if self.bounds.is_empty() {
            return;
        }
        let icon_size = self
            .style
            .icon_size
            .max(0.0)
            .min(self.bounds.size.width)
            .min(self.bounds.size.height);
        let icon_y = self.bounds.origin.y + (self.bounds.size.height - icon_size) * 0.5;
        if icon_size > 0.0 {
            scene.draw_icon(PaintIcon::new(
                self.icon,
                Rect::from_xywh(self.bounds.origin.x, icon_y, icon_size, icon_size),
                self.style.icon_color,
            ));
        }

        let text_x = self.bounds.origin.x
            + icon_size
            + if icon_size > 0.0 {
                self.style.content_gap.max(0.0)
            } else {
                0.0
            };
        let text_width = (self.bounds.right() - text_x).max(0.0);
        let text_height = self
            .style
            .text_style
            .line_height()
            .max(0.0)
            .min(self.bounds.size.height);
        if self.label.is_empty() || text_width <= 0.0 || text_height <= 0.0 {
            return;
        }
        let text_y = self.bounds.origin.y + (self.bounds.size.height - text_height) * 0.5;
        let origin = Point::new(text_x, text_y);
        let bounds = crate::Size::new(text_width, text_height);
        let text = if self.spans.is_empty() {
            TextBlock::new(
                self.label.clone(),
                origin,
                bounds,
                self.style.text_style.clone(),
            )
        } else {
            TextBlock::from_spans(
                self.spans.clone(),
                origin,
                bounds,
                self.style.text_style.clone(),
            )
        };
        scene.draw_text(text);
    }
}

#[cfg(test)]
#[path = "icon_label_tests.rs"]
mod tests;
