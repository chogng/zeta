use std::path::{Path, PathBuf};

use zeta_ui_components::{
    ButtonBackgrounds, ButtonState, ButtonStyle, ContextViewAnchorPosition, ContextViewPlacement,
    Dropdown, DropdownItem, DropdownScrollConfiguration, DropdownSelection, DropdownStyle,
    InputBoxState, InteractionRegion, ScrollAxis, ScrollCommand, ScrollMetrics, ScrollState,
    SearchBox,
};
use zui::ui::{
    AccessibilityRole, AccessibilitySelection, CursorFeedback, ElementId, FocusBehavior,
    NavigationAxis, NavigationGroupId, NodeAction, UiDispatch, UiNode,
};
use zui::ui::{
    CaretVisibility, Component, ComponentContext, ComponentElement, ComputedElement, CornerRadii,
    Edges, Element, Rect, Size, TextInput, TextInputCommand, TextInputCompositionEvent,
    TextInputLayoutEngine, TextStyle, UiScene,
};

use crate::display_working_directory;
use zeta_ui_theme::UiTheme;

const WINDOW: ElementId = ElementId::scoped(1, 1);

#[path = "workspace_picker_path.rs"]
mod path_support;
use path_support::{
    canonical_directory, directory_name, home_directory, read_child_directories,
    resolve_directory_query,
};

const PATH_PICKER_SCOPE: u32 = 2;
const WORKSPACE_PATH_PICKER: ElementId = ElementId::scoped(PATH_PICKER_SCOPE, 1);
pub const WORKSPACE_PATH_SEARCH_INPUT: ElementId = ElementId::scoped(PATH_PICKER_SCOPE, 2);
const FIRST_WORKSPACE_PATH_ITEM: u32 = 3;
const PICKER_VISIBLE_ITEM_COUNT: usize = 8;
const PICKER_CONTENT_WIDTH: f32 = 320.0;
pub const PICKER_ITEM_HEIGHT: f32 = 30.0;
const PICKER_SEARCH_ROW_HEIGHT: f32 = 36.0;
const PICKER_SEARCH_INSET: f32 = 4.0;
const PICKER_VIEWPORT_MARGIN: f32 = 6.0;
const PICKER_ANCHOR_GAP: f32 = 4.0;

#[derive(Clone, Debug, Eq, PartialEq)]
enum WorkspacePathPickerAction {
    Select(PathBuf),
    Browse(PathBuf),
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct WorkspacePathPickerItem {
    label: String,
    action: Option<WorkspacePathPickerAction>,
}

#[derive(Clone, Debug, PartialEq)]
struct OpenWorkspacePathPicker {
    anchor: Rect,
    directory: PathBuf,
    repository_root: Option<PathBuf>,
    directories: Vec<PathBuf>,
    restore_focus: Option<ElementId>,
}

/// Product-owned directory browsing state for the workspace path picker.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WorkspacePathPickerState {
    open: Option<OpenWorkspacePathPicker>,
    search_input: TextInput,
    scroll: ScrollState,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum WorkspacePathPickerActivation {
    BrowseChanged,
    SelectWorkspace(PathBuf),
}

impl WorkspacePathPickerState {
    pub fn open(
        &mut self,
        anchor: Rect,
        directory: &Path,
        repository_root: Option<&Path>,
        restore_focus: Option<ElementId>,
    ) -> std::io::Result<()> {
        let directory = canonical_directory(directory)?;
        let repository_root = repository_root
            .and_then(|root| canonical_directory(root).ok())
            .filter(|root| directory.starts_with(root));
        let directories = read_child_directories(&directory)?;
        self.search_input.take_text();
        self.open = Some(OpenWorkspacePathPicker {
            anchor,
            directory,
            repository_root,
            directories,
            restore_focus,
        });
        self.scroll = ScrollState::default();
        Ok(())
    }

    pub const fn is_open(&self) -> bool {
        self.open.is_some()
    }

    pub fn dismiss(&mut self) -> Option<ElementId> {
        self.open.take().and_then(|open| open.restore_focus)
    }

    pub const fn search_input(&self) -> &TextInput {
        &self.search_input
    }

    pub fn apply_search(&mut self, command: TextInputCommand) {
        self.search_input.apply(command);
        self.search_changed();
    }

    pub fn apply_search_composition(&mut self, event: TextInputCompositionEvent) {
        self.search_input.apply_composition(event);
        self.search_changed();
    }

    pub fn cancel_search_composition(&mut self) {
        self.search_input.cancel_composition();
    }

    pub fn selected_search_text(&self) -> Option<&str> {
        self.search_input.selected_text()
    }

    pub const fn scroll_state(&self) -> ScrollState {
        self.scroll
    }

    pub fn apply_scroll(&mut self, command: ScrollCommand, metrics: ScrollMetrics) -> bool {
        self.scroll.apply(command, metrics, ScrollAxis::Vertical)
    }

    pub fn ensure_item_visible(&mut self, index: usize, metrics: ScrollMetrics) -> bool {
        self.apply_scroll(
            ScrollCommand::EnsureVisible(Rect::from_xywh(
                0.0,
                index as f32 * PICKER_ITEM_HEIGHT,
                metrics.content().width,
                PICKER_ITEM_HEIGHT,
            )),
            metrics,
        )
    }

    pub fn first_action_id(&self) -> Option<ElementId> {
        self.items()
            .iter()
            .enumerate()
            .find_map(|(index, item)| item.action.as_ref().map(|_| workspace_path_item_id(index)))
    }

    pub fn is_picker_element(&self, id: ElementId) -> bool {
        id == WORKSPACE_PATH_PICKER
            || id == WORKSPACE_PATH_SEARCH_INPUT
            || self
                .items()
                .iter()
                .enumerate()
                .any(|(index, _)| workspace_path_item_id(index) == id)
    }

    pub fn item_index(&self, id: ElementId) -> Option<usize> {
        self.items()
            .iter()
            .enumerate()
            .find_map(|(index, _)| (workspace_path_item_id(index) == id).then_some(index))
    }

    pub fn activate(
        &mut self,
        index: usize,
    ) -> std::io::Result<Option<WorkspacePathPickerActivation>> {
        let Some(item) = self.items().get(index).cloned() else {
            return Ok(None);
        };
        let Some(action) = item.action else {
            return Ok(None);
        };
        match action {
            WorkspacePathPickerAction::Select(directory) => Ok(Some(
                WorkspacePathPickerActivation::SelectWorkspace(directory),
            )),
            WorkspacePathPickerAction::Browse(directory) => {
                self.browse(&directory)?;
                Ok(Some(WorkspacePathPickerActivation::BrowseChanged))
            }
        }
    }

    fn browse(&mut self, directory: &Path) -> std::io::Result<()> {
        let directory = canonical_directory(directory)?;
        let directories = read_child_directories(&directory)?;
        let Some(open) = self.open.as_mut() else {
            return Ok(());
        };
        open.directory = directory;
        open.directories = directories;
        self.search_input.take_text();
        self.scroll = ScrollState::default();
        Ok(())
    }

    fn items(&self) -> Vec<WorkspacePathPickerItem> {
        let Some(open) = self.open.as_ref() else {
            return Vec::new();
        };
        let raw_query = self.search_input.text().trim();
        if let Some(directory) = resolve_directory_query(&open.directory, raw_query) {
            return vec![WorkspacePathPickerItem {
                label: format!(
                    "Use path · {}",
                    display_working_directory(&directory, home_directory().as_deref())
                ),
                action: Some(WorkspacePathPickerAction::Select(directory)),
            }];
        }
        let query = raw_query.to_lowercase();
        let directories = open
            .directories
            .iter()
            .filter(|directory| {
                query.is_empty() || directory_name(directory).to_lowercase().contains(&query)
            })
            .collect::<Vec<_>>();
        if !query.is_empty() {
            if directories.is_empty() {
                return vec![WorkspacePathPickerItem {
                    label: "No matching folders".to_string(),
                    action: None,
                }];
            }
            return directories
                .into_iter()
                .map(directory_item)
                .collect::<Vec<_>>();
        }
        let home = home_directory();
        let mut items = vec![WorkspacePathPickerItem {
            label: format!(
                "Use this folder · {}",
                display_working_directory(&open.directory, home.as_deref())
            ),
            action: Some(WorkspacePathPickerAction::Select(open.directory.clone())),
        }];
        if let Some(repository_root) = open
            .repository_root
            .as_ref()
            .filter(|root| *root != &open.directory)
        {
            items.push(WorkspacePathPickerItem {
                label: format!(
                    "Git repository root · {}",
                    display_working_directory(repository_root, home.as_deref())
                ),
                action: Some(WorkspacePathPickerAction::Select(repository_root.clone())),
            });
        }
        if let Some(parent) = open.directory.parent() {
            items.push(WorkspacePathPickerItem {
                label: format!("↑ Parent · {}", directory_name(parent)),
                action: Some(WorkspacePathPickerAction::Browse(parent.to_path_buf())),
            });
        }
        items.extend(directories.into_iter().map(directory_item));
        items
    }

    fn search_changed(&mut self) {
        self.scroll = ScrollState::default();
    }
}

pub struct WorkspacePathPicker {
    dropdown: Dropdown,
    search_box: SearchBox,
    search_value: String,
    items: Vec<WorkspacePathPickerItem>,
}

impl WorkspacePathPicker {
    pub fn new(
        viewport: Rect,
        state: &WorkspacePathPickerState,
        caret_visibility: CaretVisibility,
        palette: UiTheme,
        text_layout: &mut TextInputLayoutEngine,
        dispatch: &UiDispatch,
    ) -> Option<Self> {
        let open = state.open.as_ref()?;
        let items = state.items();
        let resting_backgrounds = ButtonBackgrounds::new(zui::ui::Color::TRANSPARENT);
        let selected_backgrounds = ButtonBackgrounds::new(palette.list_active_background)
            .with_hovered(palette.list_active_background)
            .with_focused(palette.list_active_background)
            .with_pressed(palette.border);
        let button_style = ButtonStyle::new(
            resting_backgrounds,
            TextStyle::new(13.0, palette.foreground).with_line_height(18.0),
        )
        .with_selected_backgrounds(selected_backgrounds)
        .with_corner_radii(CornerRadii::uniform(2.0))
        .with_padding(Edges::new(0.0, 10.0, 0.0, 10.0));
        let dropdown_items = items
            .iter()
            .enumerate()
            .map(|(index, entry)| {
                let id = workspace_path_item_id(index);
                let state = if entry.action.is_none() {
                    ButtonState::Disabled
                } else if dispatch.is_pressed(id) {
                    ButtonState::Pressed
                } else if dispatch.is_focused(id) {
                    ButtonState::Focused
                } else if dispatch.is_hovered(id) {
                    ButtonState::Hovered
                } else {
                    ButtonState::Resting
                };
                DropdownItem::new(entry.label.clone(), state)
            })
            .collect();
        let selection = items
            .iter()
            .enumerate()
            .find_map(|(index, _)| {
                let id = workspace_path_item_id(index);
                (dispatch.is_pressed(id) || dispatch.is_hovered(id) || dispatch.is_focused(id))
                    .then_some(index)
            })
            .map(DropdownSelection::Item)
            .unwrap_or(DropdownSelection::None);
        let dropdown = Dropdown::new_scrollable(
            viewport,
            open.anchor,
            dropdown_items,
            DropdownStyle::new(
                palette.content_background,
                button_style,
                Size::new(PICKER_CONTENT_WIDTH, PICKER_ITEM_HEIGHT),
            )
            .with_corner_radii(CornerRadii::uniform(4.0))
            .with_header_height(PICKER_SEARCH_ROW_HEIGHT)
            .with_placement(
                ContextViewPlacement::new()
                    .with_position(ContextViewAnchorPosition::Before)
                    .with_gap(PICKER_ANCHOR_GAP)
                    .with_viewport_margin(PICKER_VIEWPORT_MARGIN),
            ),
            DropdownScrollConfiguration::new(
                state.scroll_state(),
                PICKER_VISIBLE_ITEM_COUNT,
                palette.picker_scroll_view_style(),
            ),
        )
        .with_selection(selection);
        let header_bounds = dropdown
            .header_bounds()
            .expect("workspace path picker reserves a search row");
        let search_bounds = Rect::from_xywh(
            header_bounds.origin.x + PICKER_SEARCH_INSET,
            header_bounds.origin.y + PICKER_SEARCH_INSET,
            (header_bounds.size.width - PICKER_SEARCH_INSET * 2.0).max(1.0),
            (header_bounds.size.height - PICKER_SEARCH_INSET * 2.0).max(1.0),
        );
        let search_state = if dispatch.is_focused(WORKSPACE_PATH_SEARCH_INPUT) {
            InputBoxState::Focused(caret_visibility)
        } else if dispatch.is_hovered(WORKSPACE_PATH_SEARCH_INPUT) {
            InputBoxState::Hovered
        } else {
            InputBoxState::Resting
        };
        let search_box = SearchBox::new(
            search_bounds,
            "Search folders...",
            search_state,
            palette.search_box_style(),
            state.search_input(),
            text_layout,
        );
        Some(Self {
            dropdown,
            search_box,
            search_value: state.search_input().text().to_string(),
            items,
        })
    }

    fn child_interaction_regions(&self) -> Vec<InteractionRegion> {
        let navigation_group = NavigationGroupId::new(WORKSPACE_PATH_PICKER);
        let mut regions = vec![
            InteractionRegion::new(
                "WorkspacePathSearchInput",
                WORKSPACE_PATH_SEARCH_INPUT,
                self.search_box.bounds(),
                AccessibilityRole::TextInput,
                "Search workspace folders",
            )
            .with_cursor(CursorFeedback::Text)
            .with_focus(FocusBehavior::TabStop)
            .with_navigation(navigation_group, NavigationAxis::Vertical)
            .with_value(&self.search_value),
        ];
        for (index, item) in self.items.iter().enumerate() {
            let Some(bounds) = self.dropdown.interactive_item_bounds(index) else {
                continue;
            };
            regions.push(
                InteractionRegion::new(
                    "WorkspacePathItem",
                    workspace_path_item_id(index),
                    bounds,
                    AccessibilityRole::MenuItem,
                    item.label.clone(),
                )
                .with_cursor(CursorFeedback::Pointer)
                .with_focus(FocusBehavior::TabStop)
                .with_action(NodeAction::Activate)
                .with_navigation(navigation_group, NavigationAxis::Vertical)
                .with_selection(if self.dropdown.selected_index() == Some(index) {
                    AccessibilitySelection::Selected
                } else {
                    AccessibilitySelection::Unselected
                }),
            );
        }
        regions
    }

    #[cfg(test)]
    pub const fn bounds(&self) -> Rect {
        self.dropdown.bounds()
    }

    pub const fn search_caret_bounds(&self) -> Option<Rect> {
        self.search_box.caret_bounds()
    }

    pub const fn item_viewport_bounds(&self) -> Rect {
        self.dropdown.item_viewport_bounds()
    }

    pub fn scroll_metrics(&self) -> Option<ScrollMetrics> {
        self.dropdown.scroll_metrics()
    }
}

impl Component for WorkspacePathPicker {
    fn element(&self) -> ComponentElement {
        Element::leaf("WorkspacePathPicker")
            .in_bounds(self.dropdown.bounds())
            .with_identity(WORKSPACE_PATH_PICKER)
    }

    fn interaction_node(&self, element: &ComputedElement) -> Option<UiNode> {
        Some(
            UiNode::new(
                WORKSPACE_PATH_PICKER,
                element.bounds(),
                AccessibilityRole::Menu,
                "Choose workspace folder",
            )
            .with_parent(WINDOW),
        )
    }

    fn compose(&self, context: &mut ComponentContext<'_, '_>, _element: &ComputedElement) {
        context.set_modal_root(WORKSPACE_PATH_PICKER);
        for region in self.child_interaction_regions() {
            context.draw_component(&region);
        }
        self.dropdown
            .draw_components_with_header(context, |context, _bounds| {
                context.draw_component(&self.search_box);
            });
    }

    fn paint(&self, scene: &mut UiScene) {
        self.dropdown.paint_with_header(scene, |scene, _bounds| {
            scene.draw_component(&self.search_box)
        });
    }
}

pub fn workspace_path_item_id(index: usize) -> ElementId {
    ElementId::scoped(
        PATH_PICKER_SCOPE,
        FIRST_WORKSPACE_PATH_ITEM.saturating_add(index as u32),
    )
}

fn directory_item(directory: &PathBuf) -> WorkspacePathPickerItem {
    WorkspacePathPickerItem {
        label: format!("› {}/", directory_name(directory)),
        action: Some(WorkspacePathPickerAction::Browse(directory.clone())),
    }
}

#[cfg(test)]
#[path = "workspace_picker_tests.rs"]
mod tests;
