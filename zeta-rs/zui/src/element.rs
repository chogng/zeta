use std::panic::Location;

use crate::CornerRadii;
use crate::Edges;
use crate::InspectionNode;
use crate::Rect;

/// Axis along which an [`Element`] arranges its direct children.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum ElementDirection {
    #[default]
    Horizontal,
    Vertical,
}

/// Declarative length resolved by the element layout engine.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum ElementLength {
    /// Consume the remaining length on the relevant axis.
    #[default]
    Fill,
    /// Request an exact logical-pixel length, subject to clipping by the parent.
    Pixels(f32),
}

impl ElementLength {
    pub const fn px(value: f32) -> Self {
        Self::Pixels(value)
    }

    fn fixed(self) -> Option<f32> {
        match self {
            Self::Fill => None,
            Self::Pixels(value) => Some(value.max(0.0)),
        }
    }
}

/// Authored box and child-flow properties for one declarative [`Element`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ElementStyle {
    direction: ElementDirection,
    width: ElementLength,
    height: ElementLength,
    padding: Option<Edges>,
    gap: Option<f32>,
    corner_radii: Option<CornerRadii>,
}

impl ElementStyle {
    const fn new(direction: ElementDirection) -> Self {
        Self {
            direction,
            width: ElementLength::Fill,
            height: ElementLength::Fill,
            padding: None,
            gap: None,
            corner_radii: None,
        }
    }

    pub const fn direction(self) -> ElementDirection {
        self.direction
    }

    pub const fn width(self) -> ElementLength {
        self.width
    }

    pub const fn height(self) -> ElementLength {
        self.height
    }

    pub const fn padding(self) -> Option<Edges> {
        self.padding
    }

    pub const fn gap(self) -> Option<f32> {
        self.gap
    }

    pub const fn corner_radii(self) -> Option<CornerRadii> {
        self.corner_radii
    }
}

/// Declarative UI node consumed by zui layout before paint and inspection.
///
/// Components describe box metrics and child flow with this type. The framework resolves the
/// tree into [`ComputedElement`] geometry; components should use that result for paint and hit
/// testing instead of separately interpreting the authored style.
#[derive(Clone, Debug, PartialEq)]
pub struct Element {
    name: &'static str,
    style: ElementStyle,
    children: Vec<Element>,
    source_file: &'static str,
    source_line: u32,
}

impl Element {
    #[track_caller]
    pub fn leaf(name: &'static str) -> Self {
        Self::new(name, ElementDirection::Horizontal)
    }

    #[track_caller]
    pub fn row(name: &'static str) -> Self {
        Self::new(name, ElementDirection::Horizontal)
    }

    #[track_caller]
    pub fn column(name: &'static str) -> Self {
        Self::new(name, ElementDirection::Vertical)
    }

    #[track_caller]
    fn new(name: &'static str, direction: ElementDirection) -> Self {
        let location = Location::caller();
        Self {
            name,
            style: ElementStyle::new(direction),
            children: Vec::new(),
            source_file: location.file(),
            source_line: location.line(),
        }
    }

    pub const fn width(mut self, width: ElementLength) -> Self {
        self.style.width = width;
        self
    }

    pub const fn height(mut self, height: ElementLength) -> Self {
        self.style.height = height;
        self
    }

    pub const fn padding(mut self, padding: Edges) -> Self {
        self.style.padding = Some(padding);
        self
    }

    pub const fn gap(mut self, gap: f32) -> Self {
        self.style.gap = Some(gap);
        self
    }

    pub const fn corner_radii(mut self, corner_radii: CornerRadii) -> Self {
        self.style.corner_radii = Some(corner_radii);
        self
    }

    pub fn child(mut self, child: Element) -> Self {
        self.children.push(child);
        self
    }

    pub fn children(mut self, children: impl IntoIterator<Item = Element>) -> Self {
        self.children.extend(children);
        self
    }

    pub fn in_bounds(self, bounds: Rect) -> ComponentElement {
        ComponentElement {
            root: self,
            bounds,
            overlay: false,
        }
    }

    pub fn in_overlay(self, bounds: Rect) -> ComponentElement {
        ComponentElement {
            root: self,
            bounds,
            overlay: true,
        }
    }
}

/// Root element and the host-provided bounds in which it must be resolved.
#[derive(Clone, Debug, PartialEq)]
pub struct ComponentElement {
    root: Element,
    bounds: Rect,
    overlay: bool,
}

impl ComponentElement {
    pub fn compute(&self) -> ComputedElement {
        compute_element(&self.root, self.bounds)
    }

    pub(crate) const fn is_overlay(&self) -> bool {
        self.overlay
    }
}

/// Resolved box geometry shared by component paint, hit testing, and inspection.
#[derive(Clone, Debug, PartialEq)]
pub struct ComputedElement {
    name: &'static str,
    bounds: Rect,
    style: ElementStyle,
    resolved_padding: Edges,
    gap_regions: Vec<Rect>,
    children: Vec<ComputedElement>,
    source_file: &'static str,
    source_line: u32,
}

impl ComputedElement {
    pub const fn name(&self) -> &'static str {
        self.name
    }

    pub const fn bounds(&self) -> Rect {
        self.bounds
    }

    pub const fn style(&self) -> ElementStyle {
        self.style
    }

    /// Returns the padding after it has been clamped to the resolved bounds.
    pub const fn resolved_padding(&self) -> Edges {
        self.resolved_padding
    }

    pub fn gap_regions(&self) -> &[Rect] {
        &self.gap_regions
    }

    pub fn children(&self) -> &[ComputedElement] {
        &self.children
    }

    pub fn child(&self, index: usize) -> Option<&ComputedElement> {
        self.children.get(index)
    }

    pub(crate) fn inspection_node(&self) -> InspectionNode {
        let mut node = InspectionNode::new(self.name, self.bounds)
            .with_authored_style(self.style)
            .with_source_location(self.source_file, self.source_line);
        if self.style.padding.is_some() {
            node = node.with_padding(self.resolved_padding);
        }
        if let Some(gap) = self.style.gap {
            node = node.with_gap_geometry(gap.max(0.0), self.gap_regions.clone());
        }
        if let Some(corner_radii) = self.style.corner_radii {
            node = node.with_corner_radii(corner_radii);
        }
        node
    }
}

fn compute_element(element: &Element, bounds: Rect) -> ComputedElement {
    let padding = resolved_padding(element.style.padding, bounds);
    let content_bounds = inset_bounds(bounds, padding);
    let gap = element.style.gap.unwrap_or(0.0).max(0.0);
    let child_count = element.children.len();
    let total_gap = gap * child_count.saturating_sub(1) as f32;
    let (available_main, available_cross) = match element.style.direction {
        ElementDirection::Horizontal => (content_bounds.size.width, content_bounds.size.height),
        ElementDirection::Vertical => (content_bounds.size.height, content_bounds.size.width),
    };
    let fixed_main = element
        .children
        .iter()
        .filter_map(|child| main_length(element.style.direction, child).fixed())
        .sum::<f32>();
    let fill_count = element
        .children
        .iter()
        .filter(|child| {
            main_length(element.style.direction, child)
                .fixed()
                .is_none()
        })
        .count();
    let fill_extent = if fill_count == 0 {
        0.0
    } else {
        ((available_main - fixed_main - total_gap).max(0.0)) / fill_count as f32
    };
    let mut offset = 0.0;
    let mut children = Vec::with_capacity(child_count);
    let mut gap_regions = Vec::with_capacity(child_count.saturating_sub(1));
    for (index, child) in element.children.iter().enumerate() {
        let main_extent = main_length(element.style.direction, child)
            .fixed()
            .unwrap_or(fill_extent);
        let cross_extent = cross_length(element.style.direction, child)
            .fixed()
            .unwrap_or(available_cross)
            .min(available_cross.max(0.0));
        let resolved_child_bounds = child_bounds(
            element.style.direction,
            content_bounds,
            offset,
            main_extent,
            cross_extent,
        );
        children.push(compute_element(child, resolved_child_bounds));
        offset += main_extent;
        if index + 1 < child_count {
            let gap_bounds = child_bounds(
                element.style.direction,
                content_bounds,
                offset,
                gap,
                available_cross,
            );
            if !gap_bounds.is_empty() {
                gap_regions.push(gap_bounds);
            }
            offset += gap;
        }
    }
    ComputedElement {
        name: element.name,
        bounds,
        style: element.style,
        resolved_padding: padding,
        gap_regions,
        children,
        source_file: element.source_file,
        source_line: element.source_line,
    }
}

fn main_length(direction: ElementDirection, element: &Element) -> ElementLength {
    match direction {
        ElementDirection::Horizontal => element.style.width,
        ElementDirection::Vertical => element.style.height,
    }
}

fn cross_length(direction: ElementDirection, element: &Element) -> ElementLength {
    match direction {
        ElementDirection::Horizontal => element.style.height,
        ElementDirection::Vertical => element.style.width,
    }
}

fn child_bounds(
    direction: ElementDirection,
    parent: Rect,
    offset: f32,
    main_extent: f32,
    cross_extent: f32,
) -> Rect {
    match direction {
        ElementDirection::Horizontal => Rect::from_xywh(
            parent.origin.x + offset,
            parent.origin.y,
            main_extent.min((parent.size.width - offset).max(0.0)),
            cross_extent,
        ),
        ElementDirection::Vertical => Rect::from_xywh(
            parent.origin.x,
            parent.origin.y + offset,
            cross_extent,
            main_extent.min((parent.size.height - offset).max(0.0)),
        ),
    }
}

fn resolved_padding(padding: Option<Edges>, bounds: Rect) -> Edges {
    let padding = padding.unwrap_or(Edges::uniform(0.0));
    let top = padding.top.max(0.0).min(bounds.size.height.max(0.0));
    let bottom = padding
        .bottom
        .max(0.0)
        .min((bounds.size.height - top).max(0.0));
    let left = padding.left.max(0.0).min(bounds.size.width.max(0.0));
    let right = padding
        .right
        .max(0.0)
        .min((bounds.size.width - left).max(0.0));
    Edges::new(top, right, bottom, left)
}

fn inset_bounds(bounds: Rect, padding: Edges) -> Rect {
    Rect::from_xywh(
        bounds.origin.x + padding.left,
        bounds.origin.y + padding.top,
        (bounds.size.width - padding.left - padding.right).max(0.0),
        (bounds.size.height - padding.top - padding.bottom).max(0.0),
    )
}

#[cfg(test)]
#[path = "element_tests.rs"]
mod tests;
