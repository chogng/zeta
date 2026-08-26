use zui::ui::GridLayout;
use zui::ui::GridLeafLayout;
use zui::ui::GridNode;
use zui::ui::GridSashLayout;
use zui::ui::Rect;

/// Geometry projection for a host-owned PaneGroup tree.
///
/// The host supplies the immutable [`GridNode`] for one frame and retains all topology, active
/// state, runtime binding, and split mutations. This wrapper gives product layouts an explicit
/// PaneGroup boundary without introducing a second geometry algorithm beside [`GridLayout`].
#[derive(Clone, Debug, PartialEq)]
pub struct PaneGroupLayout<LeafId, SplitId> {
    grid: GridLayout<LeafId, SplitId>,
}

impl<LeafId, SplitId> PaneGroupLayout<LeafId, SplitId>
where
    LeafId: Copy + Eq + std::hash::Hash,
    SplitId: Copy + Eq + std::hash::Hash,
{
    /// Resolves one host-owned PaneGroup geometry tree.
    pub fn new(bounds: Rect, root: &GridNode<LeafId, SplitId>) -> Self {
        Self {
            grid: GridLayout::new(bounds, root),
        }
    }

    /// Returns all visible Pane leaf bounds in tree order.
    pub fn leaves(&self) -> &[GridLeafLayout<LeafId>] {
        self.grid.leaves()
    }

    /// Returns all owning split sash geometries in tree order.
    pub fn sashes(&self) -> &[GridSashLayout<SplitId>] {
        self.grid.sashes()
    }

    /// Finds one Pane leaf by its host identity.
    pub fn leaf(&self, id: LeafId) -> Option<GridLeafLayout<LeafId>> {
        self.grid.leaf(id)
    }

    /// Exposes the underlying generic projection for callers that need split metadata.
    pub fn grid(&self) -> &GridLayout<LeafId, SplitId> {
        &self.grid
    }
}
