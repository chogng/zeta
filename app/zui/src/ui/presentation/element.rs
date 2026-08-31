use std::panic::Location;

use crate::ui::foundation::CornerRadii;
use crate::ui::foundation::Edges;
use crate::ui::foundation::ElementId;
use crate::ui::foundation::Rect;
use crate::ui::foundation::Size;

mod layout;

use layout::compute_element;

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
    /// Use the element's declared content size, or a container's children-derived natural size.
    Content,
}

impl ElementLength {
    pub const fn px(value: f32) -> Self {
        Self::Pixels(value)
    }
}

/// Distribution of direct children along an element's flow axis.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum JustifyContent {
    #[default]
    Start,
    Center,
    End,
    SpaceBetween,
}

/// Placement of direct children across an element's flow axis.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum AlignItems {
    #[default]
    Start,
    Center,
    End,
}

/// Authored box and child-flow properties for one declarative [`Element`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ElementStyle {
    direction: ElementDirection,
    width: ElementLength,
    height: ElementLength,
    justify_content: JustifyContent,
    align_items: AlignItems,
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
            justify_content: JustifyContent::Start,
            align_items: AlignItems::Start,
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

    pub const fn justify_content(self) -> JustifyContent {
        self.justify_content
    }

    pub const fn align_items(self) -> AlignItems {
        self.align_items
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
    content_size: Option<Size>,
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
            content_size: None,
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

    pub const fn justify_content(mut self, justify_content: JustifyContent) -> Self {
        self.style.justify_content = justify_content;
        self
    }

    pub const fn align_items(mut self, align_items: AlignItems) -> Self {
        self.style.align_items = align_items;
        self
    }

    /// Declares the natural size of content that is not represented by child elements.
    pub fn content_size(mut self, size: Size) -> Self {
        assert!(
            size.width.is_finite()
                && size.width >= 0.0
                && size.height.is_finite()
                && size.height >= 0.0,
            "Element content size must be finite and non-negative"
        );
        self.content_size = Some(size);
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
            identity: None,
            inspection_label: None,
        }
    }

    pub fn in_overlay(self, bounds: Rect) -> ComponentElement {
        ComponentElement {
            root: self,
            bounds,
            overlay: true,
            identity: None,
            inspection_label: None,
        }
    }
}

/// Root element and the host-provided bounds in which it must be resolved.
#[derive(Clone, Debug, PartialEq)]
pub struct ComponentElement {
    root: Element,
    bounds: Rect,
    overlay: bool,
    identity: Option<ElementId>,
    inspection_label: Option<String>,
}

impl ComponentElement {
    /// Associates this component root with the stable identity used by interaction and retained
    /// presentation state.
    pub const fn with_identity(mut self, identity: ElementId) -> Self {
        self.identity = Some(identity);
        self
    }

    /// Adds semantic content shown by layout inspection tools for this component root.
    pub fn with_inspection_label(mut self, label: impl Into<String>) -> Self {
        self.inspection_label = Some(label.into());
        self
    }

    pub fn compute(&self) -> ComputedElement {
        let mut computed = compute_element(&self.root, self.bounds);
        computed.identity = self.identity;
        computed.inspection_label = self.inspection_label.clone();
        computed
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
    identity: Option<ElementId>,
    inspection_label: Option<String>,
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

    /// Returns the stable identity shared with interaction and retained presentation state.
    pub const fn identity(&self) -> Option<ElementId> {
        self.identity
    }

    pub(crate) fn inspection_label(&self) -> Option<&str> {
        self.inspection_label.as_deref()
    }

    pub(crate) const fn source_file(&self) -> &'static str {
        self.source_file
    }

    pub(crate) const fn source_line(&self) -> u32 {
        self.source_line
    }
}

#[cfg(test)]
#[path = "element_tests.rs"]
mod tests;
