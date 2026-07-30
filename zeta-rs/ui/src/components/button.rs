use zeta_icons::Icon;

use crate::{
    Border, Color, Component, CornerRadii, Edges, PaintRect, Point, Rect, TextBlock, TextStyle,
    UiScene,
};

use super::icon_label::{IconLabel, IconLabelStyle};

/// Visual interaction state selected by a button's host.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum ButtonState {
    #[default]
    Resting,
    Hovered,
    Pressed,
}

/// State-dependent background colors for a button.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct ButtonBackgrounds {
    resting: Color,
    hovered: Color,
    pressed: Color,
}

impl ButtonBackgrounds {
    pub const fn new(resting: Color, hovered: Color, pressed: Color) -> Self {
        Self {
            resting,
            hovered,
            pressed,
        }
    }

    const fn for_state(self, state: ButtonState) -> Color {
        match state {
            ButtonState::Resting => self.resting,
            ButtonState::Hovered => self.hovered,
            ButtonState::Pressed => self.pressed,
        }
    }
}

/// Presentation contract shared by text buttons and text-with-icon buttons.
#[derive(Clone, Debug, PartialEq)]
pub struct ButtonStyle {
    backgrounds: ButtonBackgrounds,
    border: Border,
    corner_radii: CornerRadii,
    padding: Edges,
    text_style: TextStyle,
    icon_size: f32,
    content_gap: f32,
}

impl ButtonStyle {
    pub fn new(backgrounds: ButtonBackgrounds, text_style: TextStyle) -> Self {
        Self {
            backgrounds,
            border: Border::default(),
            corner_radii: CornerRadii::uniform(0.0),
            padding: Edges::uniform(8.0),
            text_style,
            icon_size: 16.0,
            content_gap: 6.0,
        }
    }

    pub const fn with_border(mut self, border: Border) -> Self {
        self.border = border;
        self
    }

    pub const fn with_corner_radii(mut self, corner_radii: CornerRadii) -> Self {
        self.corner_radii = corner_radii;
        self
    }

    pub const fn with_padding(mut self, padding: Edges) -> Self {
        self.padding = padding;
        self
    }

    pub const fn with_icon_size(mut self, icon_size: f32) -> Self {
        self.icon_size = icon_size;
        self
    }

    pub const fn with_content_gap(mut self, content_gap: f32) -> Self {
        self.content_gap = content_gap;
        self
    }
}

/// A reusable button that paints its background, optional symbolic icon, and label.
#[derive(Clone, Debug, PartialEq)]
pub struct Button {
    bounds: Rect,
    label: String,
    state: ButtonState,
    style: ButtonStyle,
    icon: Option<Icon>,
}

impl Button {
    pub fn new(
        bounds: Rect,
        label: impl Into<String>,
        state: ButtonState,
        style: ButtonStyle,
    ) -> Self {
        Self {
            bounds,
            label: label.into(),
            state,
            style,
            icon: None,
        }
    }

    pub const fn with_icon(mut self, icon: Icon) -> Self {
        self.icon = Some(icon);
        self
    }

    pub const fn bounds(&self) -> Rect {
        self.bounds
    }
}

impl Component for Button {
    fn paint(&self, scene: &mut UiScene) {
        scene.draw_rect(
            PaintRect::new(self.bounds, self.style.backgrounds.for_state(self.state))
                .with_border(self.style.border)
                .with_corner_radii(self.style.corner_radii),
        );

        let content = content_bounds(self.bounds, self.style.padding);
        if content.is_empty() {
            return;
        }
        if let Some(icon) = self.icon {
            let label = IconLabel::new(
                content,
                icon,
                self.label.clone(),
                IconLabelStyle::new(self.style.text_style.clone())
                    .with_icon_size(self.style.icon_size)
                    .with_content_gap(self.style.content_gap),
            );
            label.paint(scene);
            return;
        }
        let text_x = content.origin.x;
        let text_width = content.size.width;
        let text_height = self
            .style
            .text_style
            .line_height()
            .max(0.0)
            .min(content.size.height);
        if self.label.is_empty() || text_width <= 0.0 || text_height <= 0.0 {
            return;
        }
        let text_y = content.origin.y + (content.size.height - text_height) * 0.5;
        scene.draw_text(TextBlock::new(
            self.label.clone(),
            Point::new(text_x, text_y),
            crate::Size::new(text_width, text_height),
            self.style.text_style.clone(),
        ));
    }
}

fn content_bounds(bounds: Rect, padding: Edges) -> Rect {
    Rect::from_xywh(
        bounds.origin.x + padding.left,
        bounds.origin.y + padding.top,
        (bounds.size.width - padding.left - padding.right).max(0.0),
        (bounds.size.height - padding.top - padding.bottom).max(0.0),
    )
}

#[cfg(test)]
#[path = "button_tests.rs"]
mod tests;
