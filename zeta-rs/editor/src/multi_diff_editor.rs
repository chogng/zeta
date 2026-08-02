//! Vertically scrollable composition of multiple file DiffEditors.

use std::cell::OnceCell;

use zeta_ui::{
    Border, Color, Component, ComponentElement, CornerRadii, Edges, Element, FontFamily,
    FontWeight, ListContentPadding, PaintRect, Point, Rect, ScrollAxis, ScrollMetrics, ScrollState,
    ScrollView, ScrollViewStyle, ScrollbarPresentation, ScrollbarStyle, TextBlock, TextStyle,
    UiScene, VirtualListLayout,
};

use crate::{
    DiffEditor, DiffEditorDocument, DiffEditorFoldState, DiffEditorLabels, DiffEditorPresentation,
    DiffEditorState, DiffEditorStyle,
};

const DEFAULT_SECTION_GAP: f32 = 8.0;
const FILE_HEADER_HEIGHT: f32 = 32.0;
const FILE_HEADER_PADDING: f32 = 10.0;
const CARD_INSET: f32 = 8.0;
const CARD_PADDING: f32 = 8.0;
const CARD_BORDER_WIDTH: f32 = 1.0;
const CARD_CORNER_RADIUS: f32 = 6.0;

/// One file section projected by [`MultiDiffEditor`].
///
/// Product hosts retain file identity, document lifetime, and the per-file
/// [`DiffEditorState`]. This borrowed item only binds them for one presentation.
#[derive(Clone)]
pub struct MultiDiffEditorItem<'a> {
    file_name: &'a str,
    document: &'a DiffEditorDocument,
    editor_state: DiffEditorState,
    labels: DiffEditorLabels<'a>,
}

impl<'a> MultiDiffEditorItem<'a> {
    pub const fn new(
        file_name: &'a str,
        document: &'a DiffEditorDocument,
        editor_state: DiffEditorState,
        labels: DiffEditorLabels<'a>,
    ) -> Self {
        Self {
            file_name,
            document,
            editor_state,
            labels,
        }
    }

    pub const fn file_name(&self) -> &'a str {
        self.file_name
    }
}

/// One visible per-file unchanged-region control for product input routing.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MultiDiffEditorFoldControl {
    item_index: usize,
    region_index: usize,
    line_count: usize,
    bounds: Rect,
    state: DiffEditorFoldState,
}

impl MultiDiffEditorFoldControl {
    pub const fn item_index(self) -> usize {
        self.item_index
    }

    pub const fn region_index(self) -> usize {
        self.region_index
    }

    pub const fn line_count(self) -> usize {
        self.line_count
    }

    pub const fn bounds(self) -> Rect {
        self.bounds
    }

    pub const fn state(self) -> DiffEditorFoldState {
        self.state
    }
}

/// Reusable section metrics for one exact item/state snapshot and diff presentation.
///
/// Product hosts that process high-frequency scrolling may retain this value until items or their
/// [`DiffEditorState`] change, avoiding repeated measurement for every input delta.
#[derive(Clone, Debug, PartialEq)]
pub struct MultiDiffEditorLayout {
    sections: VirtualListLayout,
    presentation: DiffEditorPresentation,
}

impl MultiDiffEditorLayout {
    pub fn content_height(&self) -> f32 {
        self.sections.content_extent()
    }
}

impl Default for MultiDiffEditorLayout {
    fn default() -> Self {
        Self {
            sections: VirtualListLayout::variable([]),
            presentation: DiffEditorPresentation::default(),
        }
    }
}

/// Visual tokens and section geometry owned by [`MultiDiffEditor`].
#[derive(Clone, Debug, PartialEq)]
pub struct MultiDiffEditorStyle {
    surface: Color,
    file_header: Color,
    divider: Color,
    file_name: TextStyle,
    diff_editor: DiffEditorStyle,
    scroll_view: ScrollViewStyle,
    section_gap: f32,
    section_horizontal_inset: f32,
    content_vertical_padding: f32,
    section_border_width: f32,
    section_corner_radii: CornerRadii,
}

/// Resolved visual inputs used to construct a MultiDiffEditor style.
#[derive(Clone, Debug, PartialEq)]
pub struct MultiDiffEditorPalette {
    pub surface: Color,
    pub file_header: Color,
    pub divider: Color,
    pub file_name: TextStyle,
    pub diff_editor: DiffEditorStyle,
    pub scroll_view: ScrollViewStyle,
}

impl MultiDiffEditorStyle {
    pub fn light() -> Self {
        Self::new(MultiDiffEditorPalette {
            surface: Color::WHITE,
            file_header: Color::rgb(246, 246, 247),
            divider: Color::rgb(222, 222, 224),
            file_name: TextStyle::new(12.0, Color::rgb(38, 38, 41))
                .with_family(FontFamily::Monospace)
                .with_weight(FontWeight::Bold)
                .with_line_height(18.0),
            diff_editor: DiffEditorStyle::light(),
            scroll_view: ScrollViewStyle::new(
                ScrollbarStyle::new(Color::TRANSPARENT, Color::rgba(126, 126, 132, 128))
                    .with_hovered_colors(
                        Color::rgba(230, 230, 232, 96),
                        Color::rgba(104, 104, 110, 184),
                    )
                    .with_active_colors(
                        Color::rgba(222, 222, 225, 128),
                        Color::rgba(82, 82, 88, 220),
                    ),
            ),
        })
    }

    pub fn new(palette: MultiDiffEditorPalette) -> Self {
        Self {
            surface: palette.surface,
            file_header: palette.file_header,
            divider: palette.divider,
            file_name: palette.file_name,
            diff_editor: palette.diff_editor,
            scroll_view: palette.scroll_view,
            section_gap: DEFAULT_SECTION_GAP,
            section_horizontal_inset: 0.0,
            content_vertical_padding: 0.0,
            section_border_width: 0.0,
            section_corner_radii: CornerRadii::uniform(0.0),
        }
    }

    /// Light presentation with inset, bordered file cards for narrow drawers.
    pub fn light_cards() -> Self {
        Self::light().cards()
    }

    /// Applies the inset, border, and corner geometry used by narrow drawer cards.
    pub fn cards(self) -> Self {
        Self {
            section_horizontal_inset: CARD_INSET,
            content_vertical_padding: CARD_PADDING,
            section_border_width: CARD_BORDER_WIDTH,
            section_corner_radii: CornerRadii::uniform(CARD_CORNER_RADIUS),
            ..self
        }
    }
}

/// A scrollable editor surface that presents all changed files in one document.
///
/// Each visible section delegates its body to [`DiffEditor`]. The host owns the
/// item collection, per-file states, scrolling input, persistence, and the
/// side-by-side or unified presentation selected for the available surface.
pub struct MultiDiffEditor<'a> {
    bounds: Rect,
    items: &'a [MultiDiffEditorItem<'a>],
    state: ScrollState,
    style: MultiDiffEditorStyle,
    scrollbar_presentation: ScrollbarPresentation,
    diff_presentation: DiffEditorPresentation,
    section_layout: OnceCell<VirtualListLayout>,
    measured_layout: Option<&'a MultiDiffEditorLayout>,
}

impl<'a> MultiDiffEditor<'a> {
    pub fn new(
        bounds: Rect,
        items: &'a [MultiDiffEditorItem<'a>],
        state: ScrollState,
        style: MultiDiffEditorStyle,
    ) -> Self {
        Self {
            bounds,
            items,
            state,
            style,
            scrollbar_presentation: ScrollbarPresentation::default(),
            diff_presentation: DiffEditorPresentation::SideBySide,
            section_layout: OnceCell::new(),
            measured_layout: None,
        }
    }

    /// Selects the layout used by every visible per-file DiffEditor.
    pub fn with_diff_presentation(mut self, presentation: DiffEditorPresentation) -> Self {
        self.diff_presentation = presentation;
        self.section_layout.take();
        self.measured_layout = None;
        self
    }

    /// Reuses metrics previously returned by [`Self::measure_layout`].
    ///
    /// The host must invalidate the layout when item order, documents, per-file state, style, or
    /// presentation changes.
    pub fn with_measured_layout(mut self, layout: &'a MultiDiffEditorLayout) -> Self {
        debug_assert_eq!(layout.sections.item_count(), self.items.len());
        debug_assert_eq!(layout.presentation, self.diff_presentation);
        self.measured_layout = Some(layout);
        self
    }

    pub const fn with_scrollbar_presentation(
        mut self,
        presentation: ScrollbarPresentation,
    ) -> Self {
        self.scrollbar_presentation = presentation;
        self
    }

    pub const fn bounds(&self) -> Rect {
        self.bounds
    }

    pub fn content_height(&self) -> f32 {
        if let Some(layout) = self.measured_layout {
            return layout.content_height();
        }
        self.section_layout().content_extent()
    }

    pub fn scroll_metrics(&self) -> ScrollMetrics {
        self.scroll_view().metrics()
    }

    /// Measures reusable section geometry for the current items, state, style, and presentation.
    pub fn measure_layout(&self) -> MultiDiffEditorLayout {
        MultiDiffEditorLayout {
            sections: self.section_layout().clone(),
            presentation: self.diff_presentation,
        }
    }

    /// Returns the visible per-file fold controls after shared scrolling is applied.
    pub fn fold_controls(&self) -> Vec<MultiDiffEditorFoldControl> {
        if self.diff_presentation != DiffEditorPresentation::Unified {
            return Vec::new();
        }
        let viewport = self.scroll_view().viewport();
        let mut controls = Vec::new();
        let section_layout = self.section_layout();
        for item_index in section_layout.visible_range(viewport) {
            let item = &self.items[item_index];
            let item_bounds = section_layout
                .item_bounds(item_index, viewport)
                .expect("visible multi-diff section");
            let section = self.section_bounds(item_bounds);
            let diff_bounds = self.diff_bounds(section);
            controls.extend(
                DiffEditor::new(
                    diff_bounds,
                    item.document,
                    item.editor_state.clone(),
                    item.labels,
                    self.style.diff_editor.clone(),
                )
                .with_presentation(self.diff_presentation)
                .within_viewport(viewport.bounds())
                .fold_controls()
                .into_iter()
                .map(|control| MultiDiffEditorFoldControl {
                    item_index,
                    region_index: control.region_index(),
                    line_count: control.line_count(),
                    bounds: control.bounds(),
                    state: control.state(),
                }),
            );
        }
        controls
    }

    fn section_bounds(&self, item_bounds: Rect) -> Rect {
        Rect::from_xywh(
            item_bounds.origin.x + self.style.section_horizontal_inset,
            item_bounds.origin.y,
            (self.bounds.size.width - self.style.section_horizontal_inset * 2.0).max(0.0),
            item_bounds.size.height,
        )
    }

    pub fn scroll_view(&self) -> ScrollView {
        ScrollView::new(
            self.bounds,
            zeta_ui::Size::new(self.bounds.size.width, self.content_height()),
            self.state,
            ScrollAxis::Vertical,
            self.style.scroll_view,
        )
        .with_scrollbar_presentation(self.scrollbar_presentation)
    }

    fn section_height(&self, item: &MultiDiffEditorItem<'_>) -> f32 {
        self.style.section_border_width * 2.0
            + FILE_HEADER_HEIGHT
            + DiffEditor::new(
                Rect::from_xywh(0.0, 0.0, self.bounds.size.width, 0.0),
                item.document,
                item.editor_state.clone(),
                item.labels,
                self.style.diff_editor.clone(),
            )
            .with_presentation(self.diff_presentation)
            .content_height()
    }

    fn section_layout(&self) -> &VirtualListLayout {
        if let Some(layout) = self.measured_layout {
            return &layout.sections;
        }
        self.section_layout.get_or_init(|| {
            VirtualListLayout::variable(self.items.iter().map(|item| self.section_height(item)))
                .with_item_gap(self.style.section_gap)
                .with_content_padding(ListContentPadding::symmetric(
                    self.style.content_vertical_padding,
                ))
        })
    }

    fn diff_bounds(&self, section: Rect) -> Rect {
        let header_bottom = section.origin.y + self.style.section_border_width + FILE_HEADER_HEIGHT;
        Rect::from_xywh(
            section.origin.x + self.style.section_border_width,
            header_bottom,
            (section.size.width - self.style.section_border_width * 2.0).max(0.0),
            (section.bottom() - header_bottom - self.style.section_border_width).max(0.0),
        )
    }
}

impl Component for MultiDiffEditor<'_> {
    fn element(&self) -> ComponentElement {
        Element::leaf("MultiDiffEditor").in_bounds(self.bounds)
    }

    fn paint(&self, scene: &mut UiScene) {
        if self.bounds.is_empty() {
            return;
        }
        scene.draw_rect(PaintRect::new(self.bounds, self.style.surface));
        self.scroll_view().draw(scene, |scene, viewport| {
            let section_layout = self.section_layout();
            for item_index in section_layout.visible_range(viewport) {
                let item = &self.items[item_index];
                let item_bounds = section_layout
                    .item_bounds(item_index, viewport)
                    .expect("visible multi-diff section");
                let section = self.section_bounds(item_bounds);
                scene.draw_rect(
                    PaintRect::new(section, self.style.surface)
                        .with_border(Border::new(
                            Edges::uniform(self.style.section_border_width),
                            self.style.divider,
                        ))
                        .with_corner_radii(self.style.section_corner_radii),
                );
                let header = Rect::from_xywh(
                    section.origin.x + self.style.section_border_width,
                    section.origin.y + self.style.section_border_width,
                    (section.size.width - self.style.section_border_width * 2.0).max(0.0),
                    FILE_HEADER_HEIGHT.min(section.size.height),
                );
                scene.draw_rect(PaintRect::new(header, self.style.file_header).with_border(
                    Border::new(Edges::new(0.0, 0.0, 1.0, 0.0), self.style.divider),
                ));
                scene.draw_text(TextBlock::new(
                    item.file_name,
                    Point::new(header.origin.x + FILE_HEADER_PADDING, header.origin.y + 7.0),
                    zeta_ui::Size::new(
                        (header.size.width - FILE_HEADER_PADDING * 2.0).max(1.0),
                        18.0,
                    ),
                    self.style.file_name.clone(),
                ));
                let diff_bounds = self.diff_bounds(section);
                scene.draw_component(
                    &DiffEditor::new(
                        diff_bounds,
                        item.document,
                        item.editor_state.clone(),
                        item.labels,
                        self.style.diff_editor.clone(),
                    )
                    .with_presentation(self.diff_presentation)
                    .within_viewport(viewport.bounds()),
                );
            }
        });
    }
}

#[cfg(test)]
#[path = "multi_diff_editor_tests.rs"]
mod tests;
