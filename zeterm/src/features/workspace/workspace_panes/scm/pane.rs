use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::time::Instant;

use zeta_editor::{
    DiffEditorDocument, DiffEditorLabels, DiffEditorPresentation, DiffEditorState, MultiDiffEditor,
    MultiDiffEditorItem, MultiDiffEditorItemIdentity, MultiDiffEditorLayout, MultiDiffEditorStyle,
};
use zeta_ui::{
    AccessibilityRole, Border, Component, ComponentContext, ComponentElement, ComputedElement,
    Edges, Element, ElementId, PaintRect, Rect, ScrollAxis, ScrollCommand, ScrollDelta,
    ScrollMetrics, ScrollState, ScrollbarController, ScrollbarDrag, ScrollbarPart,
    ScrollbarPointerPresence, ScrollbarPresentation, Size, TextBlock, TextStyle, UiNode, UiScene,
};

use super::ScmDiff;
use super::ScmPaneStyle;
use crate::shell_interaction::{
    AGENT_EDITOR_PANE, AGENT_SIDEBAR, MULTI_DIFF_EDITOR, MULTI_DIFF_SCROLLBAR,
};

const EMPTY_STATE_PADDING: f32 = 12.0;

/// One changed file and the retained state of its DiffEditor section.
pub struct EditorDiff {
    file_name: String,
    original_label: String,
    modified_label: String,
    document: DiffEditorDocument,
    editor_state: DiffEditorState,
    identity: MultiDiffEditorItemIdentity,
}

impl EditorDiff {
    fn item(&self) -> MultiDiffEditorItem<'_> {
        MultiDiffEditorItem::new(
            &self.file_name,
            &self.document,
            self.editor_state.clone(),
            DiffEditorLabels::new(&self.original_label, &self.modified_label),
        )
        .with_identity(self.identity)
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
pub struct EditorPaneState {
    diffs: Vec<EditorDiff>,
    scroll_state: ScrollState,
    scrollbar: ScrollbarController,
    scrollbar_capture: Option<ScrollbarCapture>,
    measured_layout: MultiDiffEditorLayout,
    style: MultiDiffEditorStyle,
    diff_identities: BTreeMap<String, MultiDiffEditorItemIdentity>,
    next_identity_slot: u32,
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
            diff_identities: BTreeMap::new(),
            next_identity_slot: 1,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum ScrollbarCapture {
    Thumb(ScrollbarDrag),
    Track,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ScrollbarPointerOutcome {
    pub handled: bool,
    pub presentation_changed: bool,
}

impl EditorPaneState {
    pub fn set_style(&mut self, style: MultiDiffEditorStyle) {
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

    pub fn toggle_fold_for_element(&mut self, id: ElementId) -> bool {
        let mut toggled = false;
        for diff in &mut self.diffs {
            for region_index in 0..diff.document.diff().rows().len() {
                if diff.identity.fold_id(region_index) == Some(id) {
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

    pub fn replace_diffs(&mut self, diffs: &[ScmDiff]) -> Vec<MultiDiffEditorItemIdentity> {
        let mut next_diffs = Vec::with_capacity(diffs.len());
        for diff in diffs {
            let file_name = diff.path().to_owned();
            next_diffs.push(EditorDiff {
                identity: self.identity_for_path(&file_name),
                file_name,
                original_label: "HEAD".to_string(),
                modified_label: "Working Tree".to_string(),
                document: diff.document().clone(),
                editor_state: DiffEditorState::default(),
            });
        }
        self.replace_editor_diffs(next_diffs)
    }

    fn identity_for_path(&mut self, path: &str) -> MultiDiffEditorItemIdentity {
        if let Some(identity) = self.diff_identities.get(path) {
            return *identity;
        }
        let slot = self.next_identity_slot;
        self.next_identity_slot = self
            .next_identity_slot
            .checked_add(1)
            .expect("changed-file identity space exhausted");
        let identity = MultiDiffEditorItemIdentity::from_slot(slot);
        self.diff_identities.insert(path.to_owned(), identity);
        identity
    }

    fn replace_editor_diffs(
        &mut self,
        next_diffs: Vec<EditorDiff>,
    ) -> Vec<MultiDiffEditorItemIdentity> {
        let previous_states = self
            .diffs
            .drain(..)
            .map(|diff| (diff.file_name, (diff.editor_state, diff.identity)))
            .collect::<BTreeMap<_, _>>();
        let next_paths = next_diffs
            .iter()
            .map(|diff| diff.file_name.as_str())
            .collect::<BTreeSet<_>>();
        let removed_identities = previous_states
            .iter()
            .filter_map(|(path, (_, identity))| {
                (!next_paths.contains(path.as_str())).then_some(*identity)
            })
            .collect();
        self.diffs = next_diffs
            .into_iter()
            .map(|mut diff| {
                if let Some((state, _)) = previous_states.get(&diff.file_name) {
                    diff.editor_state = state.clone();
                }
                diff
            })
            .collect();
        self.scroll_state = ScrollState::default();
        self.scrollbar = ScrollbarController::default();
        self.scrollbar_capture = None;
        self.remeasure();
        removed_identities
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
        let file_name = file_name.into();
        let identity = self.identity_for_path(&file_name);
        self.diffs.push(EditorDiff {
            identity,
            file_name,
            original_label: original_label.into(),
            modified_label: modified_label.into(),
            document: DiffEditorDocument::new(document, zeta_editor::CodeEditorLanguage::PlainText),
            editor_state: DiffEditorState::default(),
        });
        self.remeasure();
    }

    #[cfg(test)]
    pub(crate) fn replace_test_diffs(
        &mut self,
        diffs: Vec<(String, zeta_diff::DiffDocument)>,
    ) -> Vec<MultiDiffEditorItemIdentity> {
        let mut next_diffs = Vec::with_capacity(diffs.len());
        for (file_name, document) in diffs {
            next_diffs.push(EditorDiff {
                identity: self.identity_for_path(&file_name),
                file_name,
                original_label: "HEAD".to_string(),
                modified_label: "Working Tree".to_string(),
                document: DiffEditorDocument::new(
                    document,
                    zeta_editor::CodeEditorLanguage::PlainText,
                ),
                editor_state: DiffEditorState::default(),
            });
        }
        self.replace_editor_diffs(next_diffs)
    }

    fn items(&self) -> Vec<MultiDiffEditorItem<'_>> {
        self.diffs.iter().map(EditorDiff::item).collect()
    }

    pub fn scroll(&mut self, delta: f32, viewport: Size, now: Instant) -> bool {
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

    pub fn scrollbar_pointer_moved(
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

    pub fn press_scrollbar(
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

    pub fn release_scrollbar(
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

    pub fn scrollbar_pointer_left(&mut self, now: Instant) -> bool {
        if self.scrollbar_capture.is_some() {
            return false;
        }
        let previous = self.scrollbar.presentation();
        self.scrollbar
            .pointer_presence(ScrollbarPointerPresence::Outside, now);
        self.scrollbar.presentation() != previous
    }

    pub fn cancel_scrollbar_interaction(&mut self) {
        self.scrollbar_capture = None;
        self.scrollbar.cancel();
    }

    pub fn advance_scrollbar(&mut self, now: Instant) -> bool {
        self.scrollbar.advance(now)
    }

    pub const fn scrollbar_deadline(&self) -> Option<Instant> {
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
pub struct EditorPane<'a> {
    bounds: Rect,
    state: &'a EditorPaneState,
    style: ScmPaneStyle,
}

impl<'a> EditorPane<'a> {
    pub const fn new(bounds: Rect, state: &'a EditorPaneState, style: ScmPaneStyle) -> Self {
        Self {
            bounds,
            state,
            style,
        }
    }

    fn interaction_node_for_bounds(&self, bounds: Rect) -> UiNode {
        UiNode::new(
            AGENT_EDITOR_PANE,
            bounds,
            AccessibilityRole::Group,
            "Changed files editor",
        )
        .with_parent(AGENT_SIDEBAR)
    }

    fn paint_surface(&self, scene: &mut UiScene, bounds: Rect) {
        scene.draw_rect(
            PaintRect::new(bounds, self.style.surface).with_border(Border::new(
                Edges::new(1.0, 0.0, 0.0, 0.0),
                self.style.border,
            )),
        );
    }

    fn paint_empty_state(&self, scene: &mut UiScene, bounds: Rect) {
        scene.draw_text(TextBlock::new(
            "No changed files",
            zeta_ui::Point::new(
                bounds.origin.x + EMPTY_STATE_PADDING,
                bounds.origin.y + EMPTY_STATE_PADDING,
            ),
            zeta_ui::Size::new(
                (bounds.size.width - EMPTY_STATE_PADDING * 2.0).max(1.0),
                18.0,
            ),
            TextStyle::new(12.0, self.style.text_muted).with_line_height(18.0),
        ));
    }
}

impl Component for EditorPane<'_> {
    fn element(&self) -> ComponentElement {
        Element::leaf("EditorPane")
            .in_bounds(self.bounds)
            .with_identity(AGENT_EDITOR_PANE)
    }

    fn interaction_node(&self, element: &ComputedElement) -> Option<UiNode> {
        Some(self.interaction_node_for_bounds(element.bounds()))
    }

    fn compose(&self, context: &mut ComponentContext<'_, '_>, element: &ComputedElement) {
        let bounds = element.bounds();
        self.paint_surface(context.scene_mut(), bounds);
        if self.state.diffs().is_empty() {
            self.paint_empty_state(context.scene_mut(), bounds);
            return;
        }
        let items = self.state.items();
        let editor =
            MultiDiffEditor::new(bounds, &items, self.state.scroll_state, self.state.style())
                .with_diff_presentation(DiffEditorPresentation::Unified)
                .with_measured_layout(&self.state.measured_layout)
                .with_scrollbar_presentation(self.state.scrollbar_presentation())
                .with_identity(MULTI_DIFF_EDITOR)
                .with_scrollbar_identity(MULTI_DIFF_SCROLLBAR);
        context.draw_component(&editor);
    }

    fn paint(&self, scene: &mut UiScene) {
        self.paint_surface(scene, self.bounds);
        if self.state.diffs().is_empty() {
            self.paint_empty_state(scene, self.bounds);
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
#[path = "pane_tests.rs"]
mod tests;
