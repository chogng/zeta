//! Vertically scrollable composition of multiple file DiffEditors.

use zeta_diff::DiffDocument;
use zeta_ui::{
    Border, Color, Component, Edges, FontFamily, FontWeight, PaintRect, Point, Rect, ScrollAxis,
    ScrollMetrics, ScrollState, ScrollView, ScrollViewStyle, ScrollViewport, ScrollbarPresentation,
    ScrollbarStyle, TextBlock, TextStyle, UiScene,
};

use crate::{DiffEditor, DiffEditorLabels, DiffEditorState, DiffEditorStyle};

const DEFAULT_SECTION_GAP: f32 = 8.0;
const FILE_HEADER_HEIGHT: f32 = 32.0;
const FILE_HEADER_PADDING: f32 = 10.0;

/// One file section projected by [`MultiDiffEditor`].
///
/// Product hosts retain file identity, document lifetime, and the per-file
/// [`DiffEditorState`]. This borrowed item only binds them for one presentation.
#[derive(Clone, Copy)]
pub struct MultiDiffEditorItem<'a> {
    file_name: &'a str,
    document: &'a DiffDocument,
    editor_state: DiffEditorState,
    labels: DiffEditorLabels<'a>,
}

impl<'a> MultiDiffEditorItem<'a> {
    pub const fn new(
        file_name: &'a str,
        document: &'a DiffDocument,
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

    pub const fn file_name(self) -> &'a str {
        self.file_name
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
}

impl MultiDiffEditorStyle {
    pub fn light() -> Self {
        Self {
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
            section_gap: DEFAULT_SECTION_GAP,
        }
    }
}

/// A scrollable editor surface that presents all changed files in one document.
///
/// Each visible section delegates its body to [`DiffEditor`], which in turn
/// composes two shared CodeEditor panes. The host owns the item collection,
/// per-file states, scrolling input, and persistence.
pub struct MultiDiffEditor<'a> {
    bounds: Rect,
    items: &'a [MultiDiffEditorItem<'a>],
    state: ScrollState,
    style: MultiDiffEditorStyle,
    scrollbar_presentation: ScrollbarPresentation,
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
        }
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
        self.items
            .iter()
            .map(|item| self.section_height(item))
            .sum::<f32>()
            + self.items.len().saturating_sub(1) as f32 * self.style.section_gap
    }

    pub fn scroll_metrics(&self) -> ScrollMetrics {
        self.scroll_view().metrics()
    }

    fn section_content_bounds(&self, index: usize) -> Rect {
        let offset = self.items[..index]
            .iter()
            .map(|item| self.section_height(item) + self.style.section_gap)
            .sum::<f32>();
        Rect::from_xywh(
            0.0,
            offset,
            self.bounds.size.width,
            self.section_height(&self.items[index]),
        )
    }

    fn section_bounds(&self, index: usize, viewport: ScrollViewport) -> Rect {
        let content = self.section_content_bounds(index);
        Rect::from_xywh(
            viewport.content_origin().x + content.origin.x,
            viewport.content_origin().y + content.origin.y,
            content.size.width,
            content.size.height,
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
        FILE_HEADER_HEIGHT
            + DiffEditor::new(
                Rect::from_xywh(0.0, 0.0, self.bounds.size.width, 0.0),
                item.document,
                item.editor_state,
                item.labels,
                self.style.diff_editor.clone(),
            )
            .content_height()
    }
}

impl Component for MultiDiffEditor<'_> {
    fn paint(&self, scene: &mut UiScene) {
        if self.bounds.is_empty() {
            return;
        }
        scene.draw_rect(PaintRect::new(self.bounds, self.style.surface));
        self.scroll_view().draw(scene, |scene, viewport| {
            for (index, item) in self.items.iter().enumerate() {
                let content_section = self.section_content_bounds(index);
                if content_section
                    .intersection(viewport.visible_content_bounds())
                    .is_empty()
                {
                    continue;
                }
                let section = self.section_bounds(index, viewport);
                let header = Rect::from_xywh(
                    section.origin.x,
                    section.origin.y,
                    section.size.width,
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
                let diff_bounds = Rect::from_xywh(
                    section.origin.x,
                    header.bottom(),
                    section.size.width,
                    (section.size.height - header.size.height).max(0.0),
                );
                scene.draw_component(&DiffEditor::new(
                    diff_bounds,
                    item.document,
                    item.editor_state,
                    item.labels,
                    self.style.diff_editor.clone(),
                ));
            }
        });
    }
}

#[cfg(test)]
#[path = "multi_diff_editor_tests.rs"]
mod tests;
