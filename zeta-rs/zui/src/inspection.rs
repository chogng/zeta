use crate::{CornerRadii, Edges, ElementStyle, Point, Rect};

/// Stable identity for one inspection node within a single immutable UI frame.
///
/// Identities are rebuilt with each scene. Consumers must not retain them across frames; use them
/// only to walk the current inspection hierarchy.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct InspectionNodeId(usize);

/// Resolved component geometry exposed to native UI inspection tools.
///
/// Declarative elements populate both their authored style and the resolved geometry used for
/// paint. Low-level scene integrations may omit authored style when no element produced the node.
#[derive(Clone, Debug, PartialEq)]
pub struct InspectionNode {
    id: InspectionNodeId,
    parent: Option<InspectionNodeId>,
    name: &'static str,
    bounds: Rect,
    authored_style: Option<ElementStyle>,
    padding: Option<Edges>,
    gap: Option<f32>,
    gap_regions: Vec<Rect>,
    corner_radii: Option<CornerRadii>,
    layer: usize,
    source_file: &'static str,
    source_line: u32,
}

impl InspectionNode {
    pub const fn new(name: &'static str, bounds: Rect) -> Self {
        Self {
            id: InspectionNodeId(0),
            parent: None,
            name,
            bounds,
            authored_style: None,
            padding: None,
            gap: None,
            gap_regions: Vec::new(),
            corner_radii: None,
            layer: 0,
            source_file: "",
            source_line: 0,
        }
    }

    pub const fn with_padding(mut self, padding: Edges) -> Self {
        self.padding = Some(padding);
        self
    }

    pub(crate) const fn with_authored_style(mut self, style: ElementStyle) -> Self {
        self.authored_style = Some(style);
        self
    }

    /// Records the resolved spacing between this container's sibling items.
    pub const fn with_gap(mut self, gap: f32) -> Self {
        self.gap = Some(gap);
        self
    }

    /// Records the resolved gap value and the exact regions separating sibling items.
    pub fn with_gap_geometry(mut self, gap: f32, regions: Vec<Rect>) -> Self {
        self.gap = Some(gap);
        self.gap_regions = regions;
        self
    }

    pub const fn with_corner_radii(mut self, corner_radii: CornerRadii) -> Self {
        self.corner_radii = Some(corner_radii);
        self
    }

    pub(crate) const fn with_source_location(
        mut self,
        source_file: &'static str,
        source_line: u32,
    ) -> Self {
        self.source_file = source_file;
        self.source_line = source_line;
        self
    }

    pub const fn id(&self) -> InspectionNodeId {
        self.id
    }

    pub const fn parent(&self) -> Option<InspectionNodeId> {
        self.parent
    }

    pub const fn name(&self) -> &'static str {
        self.name
    }

    pub const fn bounds(&self) -> Rect {
        self.bounds
    }

    /// Returns the declarative style that produced this resolved node.
    pub const fn authored_style(&self) -> Option<ElementStyle> {
        self.authored_style
    }

    pub const fn width(&self) -> f32 {
        self.bounds.size.width
    }

    pub const fn height(&self) -> f32 {
        self.bounds.size.height
    }

    pub const fn padding(&self) -> Option<Edges> {
        self.padding
    }

    pub const fn gap(&self) -> Option<f32> {
        self.gap
    }

    pub fn gap_regions(&self) -> &[Rect] {
        &self.gap_regions
    }

    pub fn corner_radii(&self) -> Option<CornerRadii> {
        self.corner_radii
            .map(|radii| radii.clamped_for(self.bounds.size))
    }

    pub const fn layer(&self) -> usize {
        self.layer
    }

    pub const fn source_file(&self) -> &'static str {
        self.source_file
    }

    pub const fn source_line(&self) -> u32 {
        self.source_line
    }
}

/// Per-frame hierarchy of component geometry available to a native layout inspector.
///
/// Hit testing prefers the highest scene layer, then the deepest, most recently registered
/// component at a point.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct InspectionFrame {
    nodes: Vec<InspectionNode>,
}

impl InspectionFrame {
    pub fn nodes(&self) -> &[InspectionNode] {
        &self.nodes
    }

    pub fn node(&self, id: InspectionNodeId) -> Option<&InspectionNode> {
        self.nodes.get(id.0)
    }

    pub fn target_at(&self, point: Point) -> Option<&InspectionNode> {
        self.nodes
            .iter()
            .enumerate()
            .filter(|(_, node)| node.bounds.contains(point))
            .max_by_key(|(index, node)| (node.layer, *index))
            .map(|(_, node)| node)
    }

    pub fn ancestry(&self, id: InspectionNodeId) -> Vec<&InspectionNode> {
        let mut ancestry = Vec::new();
        let mut current = self.node(id);
        for _ in 0..self.nodes.len() {
            let Some(node) = current else {
                break;
            };
            ancestry.push(node);
            current = node.parent.and_then(|parent| self.node(parent));
        }
        ancestry.reverse();
        ancestry
    }

    pub(crate) fn register(
        &mut self,
        mut node: InspectionNode,
        parent: Option<InspectionNodeId>,
        layer: usize,
        source_file: &'static str,
        source_line: u32,
    ) -> InspectionNodeId {
        let id = InspectionNodeId(self.nodes.len());
        node.id = id;
        node.parent = parent;
        node.layer = layer;
        if node.source_file.is_empty() {
            node.source_file = source_file;
            node.source_line = source_line;
        }
        self.nodes.push(node);
        id
    }
}

#[cfg(test)]
#[path = "inspection_tests.rs"]
mod tests;
