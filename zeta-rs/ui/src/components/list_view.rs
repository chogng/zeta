//! Fixed- and variable-extent virtual list geometry composed with the shared ScrollView.

use std::ops::Range;
use std::sync::Arc;

use crate::{
    Point, Rect, ScrollAxis, ScrollCommand, ScrollState, ScrollView, ScrollViewStyle,
    ScrollViewport, Size, UiScene,
};

/// Geometry for one projected list item.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ListItemLayout {
    index: usize,
    bounds: Rect,
}

impl ListItemLayout {
    pub const fn index(self) -> usize {
        self.index
    }

    pub const fn bounds(self) -> Rect {
        self.bounds
    }
}

/// Leading and trailing space around the list's item sequence.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ListContentPadding {
    before: f32,
    after: f32,
}

impl ListContentPadding {
    pub fn new(before: f32, after: f32) -> Self {
        assert_non_negative_finite(before, "List leading padding");
        assert_non_negative_finite(after, "List trailing padding");
        Self { before, after }
    }

    pub fn symmetric(value: f32) -> Self {
        Self::new(value, value)
    }

    pub const fn before(self) -> f32 {
        self.before
    }

    pub const fn after(self) -> f32 {
        self.after
    }
}

#[derive(Clone, Debug, PartialEq)]
enum ListItemExtents {
    Fixed {
        item_count: usize,
        item_extent: f32,
    },
    Variable {
        item_extents: Arc<[f32]>,
        prefix_extents: Arc<[f32]>,
    },
}

/// Platform-independent list measurement and viewport projection.
///
/// Fixed extents use direct arithmetic. Variable extents retain a prefix-height index so visible
/// ranges, hit testing, and item geometry remain logarithmic in the number of items. This type
/// owns no item content or selection; consumers retain stable identity and domain semantics.
#[derive(Clone, Debug, PartialEq)]
pub struct VirtualListLayout {
    item_extents: ListItemExtents,
    item_gap: f32,
    content_padding: ListContentPadding,
    overscan_items: usize,
}

impl VirtualListLayout {
    /// Creates a fixed-extent layout without allocating per-item geometry.
    pub fn new(item_count: usize, item_extent: f32) -> Self {
        assert_positive_finite(item_extent, "List item extent");
        Self {
            item_extents: ListItemExtents::Fixed {
                item_count,
                item_extent,
            },
            item_gap: 0.0,
            content_padding: ListContentPadding::default(),
            overscan_items: 0,
        }
    }

    /// Creates a variable-extent layout and builds its prefix-height index.
    pub fn variable(item_extents: impl IntoIterator<Item = f32>) -> Self {
        let item_extents = item_extents.into_iter().collect::<Vec<_>>();
        let mut prefix_extents = Vec::with_capacity(item_extents.len() + 1);
        prefix_extents.push(0.0);
        for &extent in &item_extents {
            assert_positive_finite(extent, "Variable list item extent");
            let prefix = prefix_extents.last().copied().unwrap_or(0.0) + extent;
            assert!(
                prefix.is_finite(),
                "Variable list cumulative extent must be finite"
            );
            prefix_extents.push(prefix);
        }
        Self {
            item_extents: ListItemExtents::Variable {
                item_extents: item_extents.into(),
                prefix_extents: prefix_extents.into(),
            },
            item_gap: 0.0,
            content_padding: ListContentPadding::default(),
            overscan_items: 0,
        }
    }

    pub fn with_item_gap(mut self, item_gap: f32) -> Self {
        assert_non_negative_finite(item_gap, "List item gap");
        self.item_gap = item_gap;
        self
    }

    pub const fn with_content_padding(mut self, content_padding: ListContentPadding) -> Self {
        self.content_padding = content_padding;
        self
    }

    pub const fn with_overscan_items(mut self, overscan_items: usize) -> Self {
        self.overscan_items = overscan_items;
        self
    }

    pub fn item_count(&self) -> usize {
        match &self.item_extents {
            ListItemExtents::Fixed { item_count, .. } => *item_count,
            ListItemExtents::Variable { item_extents, .. } => item_extents.len(),
        }
    }

    pub fn item_extent(&self, index: usize) -> Option<f32> {
        match &self.item_extents {
            ListItemExtents::Fixed {
                item_count,
                item_extent,
            } => (index < *item_count).then_some(*item_extent),
            ListItemExtents::Variable { item_extents, .. } => item_extents.get(index).copied(),
        }
    }

    pub fn content_extent(&self) -> f32 {
        let item_count = self.item_count();
        let items_extent = match &self.item_extents {
            ListItemExtents::Fixed {
                item_count,
                item_extent,
            } => *item_count as f32 * *item_extent,
            ListItemExtents::Variable { prefix_extents, .. } => {
                prefix_extents.last().copied().unwrap_or(0.0)
            }
        };
        let content_extent = self.content_padding.before
            + items_extent
            + item_count.saturating_sub(1) as f32 * self.item_gap
            + self.content_padding.after;
        assert!(
            content_extent.is_finite(),
            "List content extent must be finite"
        );
        content_extent
    }

    pub fn content_size(&self, width: f32) -> Size {
        assert_non_negative_finite(width, "List content width");
        Size::new(width, self.content_extent())
    }

    pub fn visible_range(&self, viewport: ScrollViewport) -> Range<usize> {
        self.range_for_content_bounds(viewport.visible_content_bounds(), 0)
    }

    pub fn projected_range(&self, viewport: ScrollViewport) -> Range<usize> {
        self.range_for_content_bounds(viewport.visible_content_bounds(), self.overscan_items)
    }

    pub fn item_bounds(&self, index: usize, viewport: ScrollViewport) -> Option<Rect> {
        Some(Rect::from_xywh(
            viewport.content_origin().x,
            viewport.content_origin().y + self.item_start(index)?,
            viewport.bounds().size.width,
            self.item_extent(index)?,
        ))
    }

    pub fn item_at(&self, point: Point, viewport: ScrollViewport) -> Option<usize> {
        if !viewport.bounds().contains(point) {
            return None;
        }
        let content_y = point.y - viewport.content_origin().y;
        let index = self.first_item_ending_after(content_y);
        if index >= self.item_count() || content_y < self.item_start(index)? {
            return None;
        }
        Some(index)
    }

    pub fn ensure_visible_command(&self, index: usize, width: f32) -> Option<ScrollCommand> {
        assert_non_negative_finite(width, "List item width");
        Some(ScrollCommand::EnsureVisible(Rect::from_xywh(
            0.0,
            self.item_start(index)?,
            width,
            self.item_extent(index)?,
        )))
    }

    fn item_start(&self, index: usize) -> Option<f32> {
        if index >= self.item_count() {
            return None;
        }
        let preceding_extent = match &self.item_extents {
            ListItemExtents::Fixed { item_extent, .. } => index as f32 * *item_extent,
            ListItemExtents::Variable { prefix_extents, .. } => prefix_extents[index],
        };
        Some(self.content_padding.before + preceding_extent + index as f32 * self.item_gap)
    }

    fn item_end(&self, index: usize) -> Option<f32> {
        Some(self.item_start(index)? + self.item_extent(index)?)
    }

    fn range_for_content_bounds(&self, bounds: Rect, overscan: usize) -> Range<usize> {
        if self.item_count() == 0 || bounds.is_empty() {
            return 0..0;
        }
        let first = self.first_item_ending_after(bounds.origin.y);
        let end = self.first_item_starting_at_or_after(bounds.bottom());
        first.saturating_sub(overscan).min(self.item_count())
            ..end.saturating_add(overscan).min(self.item_count())
    }

    fn first_item_ending_after(&self, coordinate: f32) -> usize {
        self.partition_items(|index| {
            self.item_end(index)
                .is_some_and(|item_end| item_end <= coordinate)
        })
    }

    fn first_item_starting_at_or_after(&self, coordinate: f32) -> usize {
        self.partition_items(|index| {
            self.item_start(index)
                .is_some_and(|item_start| item_start < coordinate)
        })
    }

    fn partition_items(&self, is_before_boundary: impl Fn(usize) -> bool) -> usize {
        let mut left = 0;
        let mut right = self.item_count();
        while left < right {
            let middle = left + (right - left) / 2;
            if is_before_boundary(middle) {
                left = middle + 1;
            } else {
                right = middle;
            }
        }
        left
    }
}

/// Virtual list composed from [`VirtualListLayout`] and [`ScrollView`].
///
/// The host owns retained [`ScrollState`], platform input routing, item identity, interaction
/// semantics, and item paint. ListView owns content extent, visible-range projection, translated
/// item bounds, clipping, and scrollbar presentation.
#[derive(Clone, Debug, PartialEq)]
pub struct ListView {
    layout: VirtualListLayout,
    scroll_view: ScrollView,
}

impl ListView {
    pub fn new(
        bounds: Rect,
        item_count: usize,
        item_extent: f32,
        state: ScrollState,
        style: ScrollViewStyle,
    ) -> Self {
        Self::from_layout(
            bounds,
            VirtualListLayout::new(item_count, item_extent),
            state,
            style,
        )
    }

    pub fn from_layout(
        bounds: Rect,
        layout: VirtualListLayout,
        state: ScrollState,
        style: ScrollViewStyle,
    ) -> Self {
        let scroll_view = ScrollView::new(
            bounds,
            layout.content_size(bounds.size.width),
            state,
            ScrollAxis::Vertical,
            style,
        );
        Self {
            layout,
            scroll_view,
        }
    }

    pub fn with_overscan_items(mut self, overscan_items: usize) -> Self {
        self.layout = self.layout.with_overscan_items(overscan_items);
        self
    }

    pub const fn layout(&self) -> &VirtualListLayout {
        &self.layout
    }

    pub const fn scroll_view(&self) -> ScrollView {
        self.scroll_view
    }

    pub fn visible_range(&self) -> Range<usize> {
        self.layout.visible_range(self.scroll_view.viewport())
    }

    pub fn item_bounds(&self, index: usize) -> Option<Rect> {
        self.layout.item_bounds(index, self.scroll_view.viewport())
    }

    pub fn item_at(&self, point: Point) -> Option<usize> {
        self.layout.item_at(point, self.scroll_view.viewport())
    }

    pub fn ensure_visible_command(&self, index: usize) -> Option<ScrollCommand> {
        self.layout
            .ensure_visible_command(index, self.scroll_view.bounds().size.width)
    }

    pub fn draw(
        &self,
        scene: &mut UiScene,
        mut draw_item: impl FnMut(&mut UiScene, ListItemLayout),
    ) {
        self.scroll_view.draw(scene, |scene, viewport| {
            for index in self.layout.projected_range(viewport) {
                let bounds = self
                    .layout
                    .item_bounds(index, viewport)
                    .expect("projected list item");
                draw_item(scene, ListItemLayout { index, bounds });
            }
        });
    }
}

fn assert_positive_finite(value: f32, label: &str) {
    assert!(
        value.is_finite() && value > 0.0,
        "{label} must be positive and finite"
    );
}

fn assert_non_negative_finite(value: f32, label: &str) {
    assert!(
        value.is_finite() && value >= 0.0,
        "{label} must be non-negative and finite"
    );
}

#[cfg(test)]
#[path = "list_view_tests.rs"]
mod tests;
