use std::time::Instant;

use zeta_diff::DiffDocument;
use zeta_editor::{
    DiffEditorLabels, DiffEditorState, MultiDiffEditor, MultiDiffEditorItem, MultiDiffEditorStyle,
};
use zeta_ui::{
    Border, Component, Edges, PaintRect, Rect, ScrollAxis, ScrollCommand, ScrollDelta, ScrollState,
    ScrollbarController, ScrollbarDrag, ScrollbarPart, ScrollbarPointerPresence,
    ScrollbarPresentation, Size, TextBlock, TextStyle, UiScene,
};
use zeta_ui_dispatch::{AccessibilityRole, InteractionFrame, UiNode};

use crate::shell_interaction::{
    AGENT_EDITOR_PANE, AGENT_SIDEBAR, MULTI_DIFF_EDITOR, MULTI_DIFF_SCROLLBAR,
};
use crate::shell_style::ShellPalette;

const EMPTY_STATE_PADDING: f32 = 12.0;

/// One changed file and the retained state of its DiffEditor section.
pub(crate) struct EditorDiff {
    file_name: String,
    original_label: String,
    modified_label: String,
    document: DiffDocument,
    editor_state: DiffEditorState,
}

impl EditorDiff {
    fn item(&self) -> MultiDiffEditorItem<'_> {
        MultiDiffEditorItem::new(
            &self.file_name,
            &self.document,
            self.editor_state,
            DiffEditorLabels::new(&self.original_label, &self.modified_label),
        )
    }

    #[allow(
        dead_code,
        reason = "the retained file viewport becomes mutable when MultiDiffEditor input is routed"
    )]
    pub(crate) const fn editor_state_mut(&mut self) -> &mut DiffEditorState {
        &mut self.editor_state
    }
}

/// Product-owned changed-file collection and retained MultiDiffEditor viewport.
#[derive(Default)]
pub(crate) struct EditorPaneState {
    diffs: Vec<EditorDiff>,
    scroll_state: ScrollState,
    scrollbar: ScrollbarController,
    scrollbar_capture: Option<ScrollbarCapture>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum ScrollbarCapture {
    Thumb(ScrollbarDrag),
    Track,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct ScrollbarPointerOutcome {
    pub(crate) handled: bool,
    pub(crate) presentation_changed: bool,
}

impl EditorPaneState {
    pub(crate) fn diffs(&self) -> &[EditorDiff] {
        &self.diffs
    }

    #[allow(
        dead_code,
        reason = "the retained file viewport becomes mutable when MultiDiffEditor input is routed"
    )]
    pub(crate) fn diff_mut(&mut self, index: usize) -> Option<&mut EditorDiff> {
        self.diffs.get_mut(index)
    }

    #[allow(
        dead_code,
        reason = "called once the authoritative changed-file projection is connected"
    )]
    pub(crate) fn open_diff(
        &mut self,
        file_name: impl Into<String>,
        original_label: impl Into<String>,
        modified_label: impl Into<String>,
        document: DiffDocument,
    ) {
        self.diffs.push(EditorDiff {
            file_name: file_name.into(),
            original_label: original_label.into(),
            modified_label: modified_label.into(),
            document,
            editor_state: DiffEditorState::default(),
        });
    }

    fn items(&self) -> Vec<MultiDiffEditorItem<'_>> {
        self.diffs.iter().map(EditorDiff::item).collect()
    }

    pub(crate) fn scroll(&mut self, delta: f32, viewport: Size, now: Instant) -> bool {
        let metrics = {
            let items = self.items();
            MultiDiffEditor::new(
                Rect::from_xywh(0.0, 0.0, viewport.width, viewport.height),
                &items,
                self.scroll_state,
                MultiDiffEditorStyle::light(),
            )
            .scroll_metrics()
        };
        let changed = self.scroll_state.apply(
            ScrollCommand::ByPixels(ScrollDelta::vertical(delta)),
            metrics,
            ScrollAxis::Vertical,
        );
        self.scrollbar.activity(now);
        changed
    }

    pub(crate) fn scrollbar_pointer_moved(
        &mut self,
        point: zeta_ui::Point,
        bounds: Rect,
        now: Instant,
    ) -> ScrollbarPointerOutcome {
        let previous_presentation = self.scrollbar.presentation();
        match self.scrollbar_capture {
            Some(ScrollbarCapture::Thumb(drag)) => {
                let metrics = self.scroll_view(bounds).metrics();
                let offset_changed =
                    self.scroll_state
                        .apply(drag.command_at(point), metrics, ScrollAxis::Vertical);
                ScrollbarPointerOutcome {
                    handled: true,
                    presentation_changed: offset_changed
                        || self.scrollbar.presentation() != previous_presentation,
                }
            }
            Some(ScrollbarCapture::Track) => ScrollbarPointerOutcome {
                handled: true,
                presentation_changed: false,
            },
            None => {
                let presence = self.scrollbar_presence(point, bounds);
                self.scrollbar.pointer_presence(presence, now);
                ScrollbarPointerOutcome {
                    handled: false,
                    presentation_changed: self.scrollbar.presentation() != previous_presentation,
                }
            }
        }
    }

    pub(crate) fn press_scrollbar(
        &mut self,
        point: zeta_ui::Point,
        bounds: Rect,
        now: Instant,
    ) -> ScrollbarPointerOutcome {
        let view = self.scroll_view(bounds);
        let Some(hit) = view.hit_test_scrollbar(point) else {
            return ScrollbarPointerOutcome::default();
        };
        let previous_presentation = self.scrollbar.presentation();
        let mut offset_changed = false;
        self.scrollbar_capture = match hit.part() {
            ScrollbarPart::Thumb => view
                .begin_scrollbar_drag(hit, point)
                .map(ScrollbarCapture::Thumb),
            ScrollbarPart::Track => {
                if let Some(command) = view.track_click_command(hit, point) {
                    offset_changed =
                        self.scroll_state
                            .apply(command, view.metrics(), ScrollAxis::Vertical);
                }
                Some(ScrollbarCapture::Track)
            }
        };
        self.scrollbar.begin_drag(now);
        ScrollbarPointerOutcome {
            handled: true,
            presentation_changed: offset_changed
                || self.scrollbar.presentation() != previous_presentation,
        }
    }

    pub(crate) fn release_scrollbar(
        &mut self,
        point: zeta_ui::Point,
        bounds: Rect,
        now: Instant,
    ) -> ScrollbarPointerOutcome {
        if self.scrollbar_capture.take().is_none() {
            return ScrollbarPointerOutcome::default();
        }
        let previous_presentation = self.scrollbar.presentation();
        let presence = self.scrollbar_presence(point, bounds);
        self.scrollbar.end_drag(presence, now);
        ScrollbarPointerOutcome {
            handled: true,
            presentation_changed: self.scrollbar.presentation() != previous_presentation,
        }
    }

    pub(crate) fn scrollbar_pointer_left(&mut self, now: Instant) -> bool {
        if self.scrollbar_capture.is_some() {
            return false;
        }
        let previous = self.scrollbar.presentation();
        self.scrollbar
            .pointer_presence(ScrollbarPointerPresence::Outside, now);
        self.scrollbar.presentation() != previous
    }

    pub(crate) fn cancel_scrollbar_interaction(&mut self) {
        self.scrollbar_capture = None;
        self.scrollbar.cancel();
    }

    pub(crate) fn advance_scrollbar(&mut self, now: Instant) -> bool {
        self.scrollbar.advance(now)
    }

    pub(crate) const fn scrollbar_deadline(&self) -> Option<Instant> {
        self.scrollbar.next_deadline()
    }

    pub(crate) fn scrollbar_presentation(&self) -> ScrollbarPresentation {
        self.scrollbar.presentation()
    }

    fn scroll_view(&self, bounds: Rect) -> zeta_ui::ScrollView {
        let items = self.items();
        MultiDiffEditor::new(
            bounds,
            &items,
            self.scroll_state,
            MultiDiffEditorStyle::light(),
        )
        .with_scrollbar_presentation(self.scrollbar.presentation())
        .scroll_view()
    }

    fn scrollbar_presence(&self, point: zeta_ui::Point, bounds: Rect) -> ScrollbarPointerPresence {
        if self.scroll_view(bounds).hit_test_scrollbar(point).is_some() {
            ScrollbarPointerPresence::Over
        } else {
            ScrollbarPointerPresence::Outside
        }
    }
}

/// Editor Pane hosting one MultiDiffEditor for all changed files.
pub(crate) struct EditorPane<'a> {
    bounds: Rect,
    state: &'a EditorPaneState,
    palette: ShellPalette,
}

impl<'a> EditorPane<'a> {
    pub(crate) const fn new(
        bounds: Rect,
        state: &'a EditorPaneState,
        palette: ShellPalette,
    ) -> Self {
        Self {
            bounds,
            state,
            palette,
        }
    }

    pub(crate) fn register_interactions(&self, frame: &mut InteractionFrame) {
        frame.register(
            UiNode::new(
                AGENT_EDITOR_PANE,
                self.bounds,
                AccessibilityRole::Group,
                "Changed files editor",
            )
            .with_parent(AGENT_SIDEBAR),
        );
        frame.register(
            UiNode::new(
                MULTI_DIFF_EDITOR,
                self.bounds,
                AccessibilityRole::Group,
                "Multiple file differences",
            )
            .with_parent(AGENT_EDITOR_PANE),
        );
        if let Some(scrollbar) = self.state.scroll_view(self.bounds).vertical_scrollbar() {
            let metrics = self.state.scroll_view(self.bounds).metrics();
            let maximum = metrics.maximum_offset().y;
            let percentage = if maximum > 0.0 {
                self.state.scroll_state.vertical_offset() / maximum * 100.0
            } else {
                0.0
            };
            frame.register(
                UiNode::new(
                    MULTI_DIFF_SCROLLBAR,
                    scrollbar.track_bounds(),
                    AccessibilityRole::ScrollBar,
                    "Changed files scrollbar",
                )
                .with_parent(MULTI_DIFF_EDITOR)
                .with_value(format!("{percentage:.0} percent")),
            );
        }
    }
}

impl Component for EditorPane<'_> {
    fn paint(&self, scene: &mut UiScene) {
        scene.draw_rect(
            PaintRect::new(self.bounds, self.palette.surface).with_border(Border::new(
                Edges::new(1.0, 0.0, 0.0, 0.0),
                self.palette.border,
            )),
        );
        if self.state.diffs().is_empty() {
            scene.draw_text(TextBlock::new(
                "No changed files",
                zeta_ui::Point::new(
                    self.bounds.origin.x + EMPTY_STATE_PADDING,
                    self.bounds.origin.y + EMPTY_STATE_PADDING,
                ),
                zeta_ui::Size::new(
                    (self.bounds.size.width - EMPTY_STATE_PADDING * 2.0).max(1.0),
                    18.0,
                ),
                TextStyle::new(12.0, self.palette.text_muted).with_line_height(18.0),
            ));
            return;
        }
        let items = self.state.items();
        scene.draw_component(
            &MultiDiffEditor::new(
                self.bounds,
                &items,
                self.state.scroll_state,
                MultiDiffEditorStyle::light(),
            )
            .with_scrollbar_presentation(self.state.scrollbar_presentation()),
        );
    }
}

#[cfg(test)]
#[path = "editor_pane_tests.rs"]
mod tests;
