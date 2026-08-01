use std::time::Instant;

use zeta_editor::{
    DiffEditorDocument, DiffEditorFoldState, DiffEditorLabels, DiffEditorPresentation,
    DiffEditorState, MultiDiffEditor, MultiDiffEditorItem, MultiDiffEditorLayout,
    MultiDiffEditorStyle,
};
use zeta_ui::{
    Border, Component, ComponentInspection, Edges, PaintRect, Rect, ScrollAxis, ScrollCommand,
    ScrollDelta, ScrollMetrics, ScrollState, ScrollbarController, ScrollbarDrag, ScrollbarPart,
    ScrollbarPointerPresence, ScrollbarPresentation, Size, TextBlock, TextStyle, UiScene,
};
use zeta_ui_dispatch::{
    AccessibilityRole, CursorFeedback, ElementId, FocusBehavior, InteractionFrame, NavigationAxis,
    NavigationGroupId, NodeAction, UiNode,
};

use crate::shell_interaction::{
    AGENT_EDITOR_PANE, AGENT_SIDEBAR, MULTI_DIFF_EDITOR, MULTI_DIFF_SCROLLBAR,
};
use crate::shell_style::ShellPalette;
use crate::workspace_context::WorkspaceDiff;

const EMPTY_STATE_PADDING: f32 = 12.0;
const DIFF_FOLD_SCOPE: u32 = 4;

/// One changed file and the retained state of its DiffEditor section.
pub(crate) struct EditorDiff {
    file_name: String,
    original_label: String,
    modified_label: String,
    document: DiffEditorDocument,
    editor_state: DiffEditorState,
}

impl EditorDiff {
    fn item(&self) -> MultiDiffEditorItem<'_> {
        MultiDiffEditorItem::new(
            &self.file_name,
            &self.document,
            self.editor_state.clone(),
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
pub(crate) struct EditorPaneState {
    diffs: Vec<EditorDiff>,
    scroll_state: ScrollState,
    scrollbar: ScrollbarController,
    scrollbar_capture: Option<ScrollbarCapture>,
    measured_layout: MultiDiffEditorLayout,
    style: MultiDiffEditorStyle,
}

impl Default for EditorPaneState {
    fn default() -> Self {
        Self {
            diffs: Vec::new(),
            scroll_state: ScrollState::default(),
            scrollbar: ScrollbarController::default(),
            scrollbar_capture: None,
            measured_layout: MultiDiffEditorLayout::default(),
            style: MultiDiffEditorStyle::light_cards(),
        }
    }
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
    pub(crate) fn set_style(&mut self, style: MultiDiffEditorStyle) {
        self.style = style;
        self.remeasure();
    }

    fn style(&self) -> MultiDiffEditorStyle {
        self.style.clone()
    }

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

    pub(crate) fn toggle_fold_for_element(&mut self, id: ElementId) -> bool {
        let mut toggled = false;
        for (item_index, diff) in self.diffs.iter_mut().enumerate() {
            for region_index in 0..diff.document.diff().rows().len() {
                if multi_diff_fold_element_id(item_index, region_index) == Some(id) {
                    diff.editor_state.toggle_unchanged_region(region_index);
                    toggled = true;
                    break;
                }
            }
            if toggled {
                break;
            }
        }
        if toggled {
            self.remeasure();
        }
        toggled
    }

    pub(crate) fn replace_diffs(&mut self, diffs: &[WorkspaceDiff]) {
        self.diffs = diffs
            .iter()
            .map(|diff| EditorDiff {
                file_name: diff.path().to_owned(),
                original_label: "HEAD".to_string(),
                modified_label: "Working Tree".to_string(),
                document: diff.document().clone(),
                editor_state: DiffEditorState::default(),
            })
            .collect();
        self.scroll_state = ScrollState::default();
        self.scrollbar = ScrollbarController::default();
        self.scrollbar_capture = None;
        self.remeasure();
    }

    #[cfg(test)]
    #[allow(
        dead_code,
        reason = "called once the authoritative changed-file projection is connected"
    )]
    pub(crate) fn open_diff(
        &mut self,
        file_name: impl Into<String>,
        original_label: impl Into<String>,
        modified_label: impl Into<String>,
        document: zeta_diff::DiffDocument,
    ) {
        self.diffs.push(EditorDiff {
            file_name: file_name.into(),
            original_label: original_label.into(),
            modified_label: modified_label.into(),
            document: DiffEditorDocument::new(document, zeta_editor::CodeEditorLanguage::PlainText),
            editor_state: DiffEditorState::default(),
        });
        self.remeasure();
    }

    fn items(&self) -> Vec<MultiDiffEditorItem<'_>> {
        self.diffs.iter().map(EditorDiff::item).collect()
    }

    pub(crate) fn scroll(&mut self, delta: f32, viewport: Size, now: Instant) -> bool {
        let metrics = ScrollMetrics::new(
            viewport,
            Size::new(viewport.width, self.measured_layout.content_height()),
        );
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
        MultiDiffEditor::new(bounds, &items, self.scroll_state, self.style())
            .with_diff_presentation(DiffEditorPresentation::Unified)
            .with_measured_layout(&self.measured_layout)
            .with_scrollbar_presentation(self.scrollbar.presentation())
            .scroll_view()
    }

    fn remeasure(&mut self) {
        let items = self.items();
        self.measured_layout = MultiDiffEditor::new(
            Rect::from_xywh(0.0, 0.0, 1.0, 0.0),
            &items,
            ScrollState::default(),
            self.style(),
        )
        .with_diff_presentation(DiffEditorPresentation::Unified)
        .measure_layout();
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
        let items = self.state.items();
        let editor = MultiDiffEditor::new(
            self.bounds,
            &items,
            self.state.scroll_state,
            self.state.style(),
        )
        .with_diff_presentation(DiffEditorPresentation::Unified)
        .with_measured_layout(&self.state.measured_layout)
        .with_scrollbar_presentation(self.state.scrollbar_presentation());
        let navigation = NavigationGroupId::new(MULTI_DIFF_EDITOR);
        for control in editor.fold_controls() {
            let Some(id) = multi_diff_fold_element_id(control.item_index(), control.region_index())
            else {
                continue;
            };
            let action = match control.state() {
                DiffEditorFoldState::Collapsed => "Show",
                DiffEditorFoldState::Expanded => "Hide",
            };
            let file_name = items[control.item_index()].file_name();
            frame.register(
                UiNode::new(
                    id,
                    control.bounds(),
                    AccessibilityRole::Button,
                    format!(
                        "{action} {} unchanged lines in {file_name}",
                        control.line_count()
                    ),
                )
                .with_parent(MULTI_DIFF_EDITOR)
                .with_cursor(CursorFeedback::Pointer)
                .with_focus(FocusBehavior::TabStop)
                .with_action(NodeAction::Activate)
                .with_navigation(navigation, NavigationAxis::Vertical),
            );
        }
        let scroll_view = editor.scroll_view();
        if let Some(scrollbar) = scroll_view.vertical_scrollbar() {
            let metrics = scroll_view.metrics();
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

fn multi_diff_fold_element_id(item_index: usize, region_index: usize) -> Option<ElementId> {
    let item_index = u16::try_from(item_index).ok()?;
    let region_index = u16::try_from(region_index).ok()?;
    let local = ((u32::from(item_index) << 16) | u32::from(region_index)).checked_add(1)?;
    Some(ElementId::scoped(DIFF_FOLD_SCOPE, local))
}

impl Component for EditorPane<'_> {
    fn inspection(&self) -> ComponentInspection {
        ComponentInspection::new("EditorPane", self.bounds)
    }

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
                self.state.style(),
            )
            .with_diff_presentation(DiffEditorPresentation::Unified)
            .with_measured_layout(&self.state.measured_layout)
            .with_scrollbar_presentation(self.state.scrollbar_presentation()),
        );
    }
}

#[cfg(test)]
#[path = "editor_pane_tests.rs"]
mod tests;
