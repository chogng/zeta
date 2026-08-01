use zeta_icons::Icon;

use crate::{Color, Component, ComponentInspection, PaintRect, Point, Rect, Size, UiScene};

use super::{Button, ButtonSelection, ButtonState, ButtonStyle};

/// Axis along which an action bar arranges its items.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum ActionBarOrientation {
    #[default]
    Horizontal,
    Vertical,
}

#[derive(Clone, Debug, PartialEq)]
enum ActionBarButtonContent {
    Label(String),
    Icon {
        icon: Icon,
        accessible_label: String,
    },
    IconAndLabel {
        icon: Icon,
        label: String,
    },
}

/// Presentation data for one runnable button inside an [`ActionBar`].
#[derive(Clone, Debug, PartialEq)]
pub struct ActionBarButton {
    content: ActionBarButtonContent,
    state: ButtonState,
    selection: ButtonSelection,
    main_axis_extent: Option<f32>,
}

impl ActionBarButton {
    pub fn label(label: impl Into<String>, state: ButtonState) -> Self {
        Self {
            content: ActionBarButtonContent::Label(label.into()),
            state,
            selection: ButtonSelection::Unselected,
            main_axis_extent: None,
        }
    }

    pub fn icon(icon: Icon, accessible_label: impl Into<String>, state: ButtonState) -> Self {
        Self {
            content: ActionBarButtonContent::Icon {
                icon,
                accessible_label: accessible_label.into(),
            },
            state,
            selection: ButtonSelection::Unselected,
            main_axis_extent: None,
        }
    }

    pub fn icon_and_label(icon: Icon, label: impl Into<String>, state: ButtonState) -> Self {
        Self {
            content: ActionBarButtonContent::IconAndLabel {
                icon,
                label: label.into(),
            },
            state,
            selection: ButtonSelection::Unselected,
            main_axis_extent: None,
        }
    }

    /// Overrides this item's extent along its ActionBar's orientation axis.
    pub const fn with_main_axis_extent(mut self, extent: f32) -> Self {
        self.main_axis_extent = Some(extent);
        self
    }

    pub const fn with_selection(mut self, selection: ButtonSelection) -> Self {
        self.selection = selection;
        self
    }

    const fn is_enabled(&self) -> bool {
        !matches!(self.state, ButtonState::Disabled)
    }

    fn paint(&self, bounds: Rect, style: &ButtonStyle, scene: &mut UiScene) {
        let button = match &self.content {
            ActionBarButtonContent::Label(label) => {
                Button::new(bounds, label.clone(), self.state, style.clone())
            }
            ActionBarButtonContent::Icon {
                icon,
                accessible_label,
            } => Button::icon(
                bounds,
                *icon,
                accessible_label.clone(),
                self.state,
                style.clone(),
            ),
            ActionBarButtonContent::IconAndLabel { icon, label } => {
                Button::icon_and_label(bounds, *icon, label.clone(), self.state, style.clone())
            }
        }
        .with_selection(self.selection);
        scene.draw_component(&button);
    }
}

/// One visual representation in an [`ActionBar`].
#[derive(Clone, Debug, PartialEq)]
pub enum ActionBarItem {
    Button(ActionBarButton),
    Separator,
}

/// Shared geometry and presentation used to arrange action bar items.
#[derive(Clone, Debug, PartialEq)]
pub struct ActionBarStyle {
    button_style: ButtonStyle,
    item_size: Size,
    gap: f32,
    separator_style: ActionBarSeparatorStyle,
}

/// Geometry and color for non-interactive action separators.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ActionBarSeparatorStyle {
    extent: f32,
    thickness: f32,
    color: Color,
}

impl ActionBarSeparatorStyle {
    pub const fn new(color: Color) -> Self {
        Self {
            extent: 8.0,
            thickness: 1.0,
            color,
        }
    }

    pub const fn with_extent(mut self, extent: f32) -> Self {
        self.extent = extent;
        self
    }

    pub const fn with_thickness(mut self, thickness: f32) -> Self {
        self.thickness = thickness;
        self
    }
}

impl ActionBarStyle {
    pub fn new(button_style: ButtonStyle, item_size: Size) -> Self {
        Self {
            button_style,
            item_size,
            gap: 0.0,
            separator_style: ActionBarSeparatorStyle::new(Color::TRANSPARENT),
        }
    }

    pub const fn with_gap(mut self, gap: f32) -> Self {
        self.gap = gap;
        self
    }

    pub const fn with_separator_style(mut self, separator_style: ActionBarSeparatorStyle) -> Self {
        self.separator_style = separator_style;
        self
    }
}

/// Presentation-only action collection with component-owned item geometry.
///
/// The action bar paints buttons and separators inside caller-provided bounds and exposes the same
/// button bounds for host hit testing. The host remains responsible for action identity, input
/// routing, focus traversal, accessibility, and command execution.
#[derive(Clone, Debug, PartialEq)]
pub struct ActionBar {
    bounds: Rect,
    orientation: ActionBarOrientation,
    items: Vec<ActionBarItem>,
    style: ActionBarStyle,
}

impl ActionBar {
    pub fn new(
        bounds: Rect,
        orientation: ActionBarOrientation,
        items: Vec<ActionBarItem>,
        style: ActionBarStyle,
    ) -> Self {
        Self {
            bounds,
            orientation,
            items,
            style,
        }
    }

    pub const fn bounds(&self) -> Rect {
        self.bounds
    }

    /// Returns the visual bounds for a button item, or `None` for a separator or missing item.
    pub fn item_bounds(&self, index: usize) -> Option<Rect> {
        let item = self.items.get(index)?;
        match item {
            ActionBarItem::Button(_) => Some(self.slot_bounds(index)),
            ActionBarItem::Separator => None,
        }
    }

    /// Returns enabled button bounds, or `None` for disabled, separator, or missing items.
    pub fn interactive_item_bounds(&self, index: usize) -> Option<Rect> {
        let ActionBarItem::Button(button) = self.items.get(index)? else {
            return None;
        };
        button.is_enabled().then(|| self.slot_bounds(index))
    }

    /// Returns the first enabled button containing `point`.
    pub fn hit_test(&self, point: Point) -> Option<usize> {
        self.items.iter().enumerate().find_map(|(index, _)| {
            self.interactive_item_bounds(index)?
                .contains(point)
                .then_some(index)
        })
    }

    fn slot_bounds(&self, target_index: usize) -> Rect {
        let mut offset = 0.0;
        for (index, item) in self.items.iter().enumerate() {
            if index == target_index {
                return self.bounds_at_offset(offset, self.item_extent(item));
            }
            offset += self.item_extent(item);
            if index + 1 < self.items.len() {
                offset += self.style.gap.max(0.0);
            }
        }
        Rect::from_xywh(self.bounds.origin.x, self.bounds.origin.y, 0.0, 0.0)
    }

    fn item_extent(&self, item: &ActionBarItem) -> f32 {
        match (self.orientation, item) {
            (_, ActionBarItem::Separator) => self.style.separator_style.extent.max(0.0),
            (ActionBarOrientation::Horizontal, ActionBarItem::Button(button)) => button
                .main_axis_extent
                .unwrap_or(self.style.item_size.width)
                .max(0.0),
            (ActionBarOrientation::Vertical, ActionBarItem::Button(button)) => button
                .main_axis_extent
                .unwrap_or(self.style.item_size.height)
                .max(0.0),
        }
    }

    fn bounds_at_offset(&self, offset: f32, extent: f32) -> Rect {
        match self.orientation {
            ActionBarOrientation::Horizontal => Rect::from_xywh(
                self.bounds.origin.x + offset,
                self.bounds.origin.y,
                extent.min((self.bounds.size.width - offset).max(0.0)),
                self.style
                    .item_size
                    .height
                    .max(0.0)
                    .min(self.bounds.size.height.max(0.0)),
            ),
            ActionBarOrientation::Vertical => Rect::from_xywh(
                self.bounds.origin.x,
                self.bounds.origin.y + offset,
                self.style
                    .item_size
                    .width
                    .max(0.0)
                    .min(self.bounds.size.width.max(0.0)),
                extent.min((self.bounds.size.height - offset).max(0.0)),
            ),
        }
    }

    fn gap_regions(&self) -> Vec<Rect> {
        let gap = self.style.gap.max(0.0);
        if gap <= 0.0 || self.items.len() < 2 {
            return Vec::new();
        }
        let mut offset = 0.0;
        self.items
            .iter()
            .take(self.items.len() - 1)
            .filter_map(|item| {
                offset += self.item_extent(item);
                let bounds = self.bounds_at_offset(offset, gap);
                offset += gap;
                (!bounds.is_empty()).then_some(bounds)
            })
            .collect()
    }

    fn separator_bounds(&self, slot: Rect) -> Rect {
        let thickness = self.style.separator_style.thickness.max(0.0);
        match self.orientation {
            ActionBarOrientation::Horizontal => Rect::from_xywh(
                slot.origin.x + (slot.size.width - thickness) * 0.5,
                slot.origin.y,
                thickness.min(slot.size.width),
                slot.size.height,
            ),
            ActionBarOrientation::Vertical => Rect::from_xywh(
                slot.origin.x,
                slot.origin.y + (slot.size.height - thickness) * 0.5,
                slot.size.width,
                thickness.min(slot.size.height),
            ),
        }
    }
}

impl Component for ActionBar {
    fn inspection(&self) -> ComponentInspection {
        ComponentInspection::new("ActionBar", self.bounds)
            .with_gap_geometry(self.style.gap.max(0.0), self.gap_regions())
    }

    fn paint(&self, scene: &mut UiScene) {
        scene.with_clip(self.bounds, |scene| {
            for (index, item) in self.items.iter().enumerate() {
                let slot = self.slot_bounds(index);
                match item {
                    ActionBarItem::Button(button) => {
                        button.paint(slot, &self.style.button_style, scene);
                    }
                    ActionBarItem::Separator => {
                        scene.draw_rect(PaintRect::new(
                            self.separator_bounds(slot),
                            self.style.separator_style.color,
                        ));
                    }
                }
            }
        });
    }
}

#[cfg(test)]
#[path = "action_bar_tests.rs"]
mod tests;
