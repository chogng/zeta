use std::io;
use std::path::{Path, PathBuf};

use zeta_ui::{
    ButtonBackgrounds, ButtonState, ButtonStyle, CaretVisibility, Component,
    ContextViewAnchorPosition, ContextViewPlacement, CornerRadii, Dropdown, DropdownItem,
    DropdownSelection, DropdownStyle, Edges, InputBoxState, Rect, SearchBox, Size, TextInput,
    TextInputCommand, TextInputCompositionEvent, TextInputLayoutEngine, TextStyle, UiScene,
};
use zeta_ui_dispatch::{
    AccessibilityRole, AccessibilitySelection, CursorFeedback, ElementId, FocusBehavior,
    InteractionFrame, NavigationAxis, NavigationGroupId, NodeAction, UiDispatch, UiNode,
};

use crate::shell_interaction::WINDOW;
use crate::shell_style::ShellPalette;
use crate::workspace_context::display_working_directory;

const PATH_PICKER_SCOPE: u32 = 2;
const WORKSPACE_PATH_PICKER: ElementId = ElementId::scoped(PATH_PICKER_SCOPE, 1);
pub(crate) const WORKSPACE_PATH_SEARCH_INPUT: ElementId = ElementId::scoped(PATH_PICKER_SCOPE, 2);
const FIRST_WORKSPACE_PATH_ITEM: u32 = 3;
const DIRECTORY_PAGE_SIZE: usize = 8;
const PICKER_CONTENT_WIDTH: f32 = 320.0;
const PICKER_ITEM_HEIGHT: f32 = 30.0;
const PICKER_SEARCH_ROW_HEIGHT: f32 = 36.0;
const PICKER_SEARCH_INSET: f32 = 4.0;
const PICKER_VIEWPORT_MARGIN: f32 = 6.0;
const PICKER_ANCHOR_GAP: f32 = 4.0;

#[derive(Clone, Debug, Eq, PartialEq)]
enum WorkspacePathPickerAction {
    SelectCurrent,
    Browse(PathBuf),
    PreviousPage,
    NextPage,
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
    directories: Vec<PathBuf>,
    page: usize,
    restore_focus: Option<ElementId>,
}

/// Product-owned directory browsing state for the workspace path picker.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct WorkspacePathPickerState {
    open: Option<OpenWorkspacePathPicker>,
    search_input: TextInput,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum WorkspacePathPickerActivation {
    BrowseChanged,
    SelectWorkspace(PathBuf),
}

impl WorkspacePathPickerState {
    pub(crate) fn open(
        &mut self,
        anchor: Rect,
        directory: &Path,
        restore_focus: Option<ElementId>,
    ) -> io::Result<()> {
        let directory = canonical_directory(directory)?;
        let directories = read_child_directories(&directory)?;
        self.search_input.take_text();
        self.open = Some(OpenWorkspacePathPicker {
            anchor,
            directory,
            directories,
            page: 0,
            restore_focus,
        });
        Ok(())
    }

    pub(crate) const fn is_open(&self) -> bool {
        self.open.is_some()
    }

    pub(crate) fn dismiss(&mut self) -> Option<ElementId> {
        self.open.take().and_then(|open| open.restore_focus)
    }

    pub(crate) const fn search_input(&self) -> &TextInput {
        &self.search_input
    }

    pub(crate) fn apply_search(&mut self, command: TextInputCommand) {
        self.search_input.apply(command);
        self.search_changed();
    }

    pub(crate) fn apply_search_composition(&mut self, event: TextInputCompositionEvent) {
        self.search_input.apply_composition(event);
        self.search_changed();
    }

    pub(crate) fn cancel_search_composition(&mut self) {
        self.search_input.cancel_composition();
    }

    pub(crate) fn selected_search_text(&self) -> Option<&str> {
        self.search_input.selected_text()
    }

    pub(crate) fn first_action_id(&self) -> Option<ElementId> {
        self.items()
            .iter()
            .enumerate()
            .find_map(|(index, item)| item.action.as_ref().map(|_| workspace_path_item_id(index)))
    }

    pub(crate) fn is_picker_element(&self, id: ElementId) -> bool {
        id == WORKSPACE_PATH_PICKER
            || id == WORKSPACE_PATH_SEARCH_INPUT
            || self
                .items()
                .iter()
                .enumerate()
                .any(|(index, _)| workspace_path_item_id(index) == id)
    }

    pub(crate) fn item_index(&self, id: ElementId) -> Option<usize> {
        self.items()
            .iter()
            .enumerate()
            .find_map(|(index, _)| (workspace_path_item_id(index) == id).then_some(index))
    }

    pub(crate) fn activate(
        &mut self,
        index: usize,
    ) -> io::Result<Option<WorkspacePathPickerActivation>> {
        let Some(item) = self.items().get(index).cloned() else {
            return Ok(None);
        };
        let Some(action) = item.action else {
            return Ok(None);
        };
        match action {
            WorkspacePathPickerAction::SelectCurrent => Ok(self.open.as_ref().map(|open| {
                WorkspacePathPickerActivation::SelectWorkspace(open.directory.clone())
            })),
            WorkspacePathPickerAction::Browse(directory) => {
                self.browse(&directory)?;
                Ok(Some(WorkspacePathPickerActivation::BrowseChanged))
            }
            WorkspacePathPickerAction::PreviousPage => {
                if let Some(open) = self.open.as_mut() {
                    open.page = open.page.saturating_sub(1);
                }
                Ok(Some(WorkspacePathPickerActivation::BrowseChanged))
            }
            WorkspacePathPickerAction::NextPage => {
                if let Some(open) = self.open.as_mut() {
                    let query = self.search_input.text().trim().to_lowercase();
                    let directory_count = open
                        .directories
                        .iter()
                        .filter(|directory| {
                            query.is_empty()
                                || directory_name(directory).to_lowercase().contains(&query)
                        })
                        .count();
                    let maximum_page = directory_count.saturating_sub(1) / DIRECTORY_PAGE_SIZE;
                    open.page = (open.page + 1).min(maximum_page);
                }
                Ok(Some(WorkspacePathPickerActivation::BrowseChanged))
            }
        }
    }

    fn browse(&mut self, directory: &Path) -> io::Result<()> {
        let directory = canonical_directory(directory)?;
        let directories = read_child_directories(&directory)?;
        let Some(open) = self.open.as_mut() else {
            return Ok(());
        };
        open.directory = directory;
        open.directories = directories;
        open.page = 0;
        self.search_input.take_text();
        Ok(())
    }

    fn items(&self) -> Vec<WorkspacePathPickerItem> {
        let Some(open) = self.open.as_ref() else {
            return Vec::new();
        };
        let query = self.search_input.text().trim().to_lowercase();
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
            return paged_directory_items(open.page, &directories);
        }
        let home = home_directory();
        let mut items = vec![WorkspacePathPickerItem {
            label: format!(
                "Use this folder · {}",
                display_working_directory(&open.directory, home.as_deref())
            ),
            action: Some(WorkspacePathPickerAction::SelectCurrent),
        }];
        if let Some(parent) = open.directory.parent() {
            items.push(WorkspacePathPickerItem {
                label: format!("↑ Parent · {}", directory_name(parent)),
                action: Some(WorkspacePathPickerAction::Browse(parent.to_path_buf())),
            });
        }
        items.extend(paged_directory_items(open.page, &directories));
        items
    }

    fn search_changed(&mut self) {
        if let Some(open) = self.open.as_mut() {
            open.page = 0;
        }
    }
}

pub(crate) struct WorkspacePathPicker {
    dropdown: Dropdown,
    search_box: SearchBox,
    search_value: String,
    items: Vec<WorkspacePathPickerItem>,
}

impl WorkspacePathPicker {
    pub(crate) fn new(
        viewport: Rect,
        state: &WorkspacePathPickerState,
        caret_visibility: CaretVisibility,
        palette: ShellPalette,
        text_layout: &mut TextInputLayoutEngine,
        dispatch: &UiDispatch,
    ) -> Option<Self> {
        let open = state.open.as_ref()?;
        let items = state.items();
        let resting_backgrounds = ButtonBackgrounds::new(zeta_ui::Color::TRANSPARENT);
        let selected_backgrounds = ButtonBackgrounds::new(palette.session_tab_highlight)
            .with_hovered(palette.session_tab_highlight)
            .with_focused(palette.session_tab_highlight)
            .with_pressed(palette.border);
        let button_style = ButtonStyle::new(
            resting_backgrounds,
            TextStyle::new(13.0, palette.text).with_line_height(18.0),
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
        let dropdown = Dropdown::new(
            viewport,
            open.anchor,
            dropdown_items,
            DropdownStyle::new(
                palette.surface,
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
            palette.session_search_style(),
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

    pub(crate) fn register_interactions(&self, frame: &mut InteractionFrame) {
        frame.register(
            UiNode::new(
                WORKSPACE_PATH_PICKER,
                self.dropdown.bounds(),
                AccessibilityRole::Menu,
                "Choose workspace folder",
            )
            .with_parent(WINDOW),
        );
        frame.set_modal_root(WORKSPACE_PATH_PICKER);
        let navigation_group = NavigationGroupId::new(WORKSPACE_PATH_PICKER);
        frame.register(
            UiNode::new(
                WORKSPACE_PATH_SEARCH_INPUT,
                self.search_box.bounds(),
                AccessibilityRole::TextInput,
                "Search workspace folders",
            )
            .with_parent(WORKSPACE_PATH_PICKER)
            .with_cursor(CursorFeedback::Text)
            .with_focus(FocusBehavior::TabStop)
            .with_navigation(navigation_group, NavigationAxis::Vertical)
            .with_value(&self.search_value),
        );
        for (index, item) in self.items.iter().enumerate() {
            let Some(bounds) = self
                .dropdown
                .interactive_item_bounds(index)
                .filter(|bounds| !bounds.is_empty())
            else {
                continue;
            };
            frame.register(
                UiNode::new(
                    workspace_path_item_id(index),
                    bounds,
                    AccessibilityRole::MenuItem,
                    item.label.clone(),
                )
                .with_parent(WORKSPACE_PATH_PICKER)
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
    }

    #[cfg(test)]
    pub(crate) const fn bounds(&self) -> Rect {
        self.dropdown.bounds()
    }

    pub(crate) const fn search_caret_bounds(&self) -> Option<Rect> {
        self.search_box.caret_bounds()
    }
}

impl Component for WorkspacePathPicker {
    fn paint(&self, scene: &mut UiScene) {
        self.dropdown
            .paint_with_header(scene, |scene, _bounds| self.search_box.paint(scene));
    }
}

fn paged_directory_items(page: usize, directories: &[&PathBuf]) -> Vec<WorkspacePathPickerItem> {
    let mut items = Vec::new();
    if page > 0 {
        items.push(WorkspacePathPickerItem {
            label: "← Previous folders".to_string(),
            action: Some(WorkspacePathPickerAction::PreviousPage),
        });
    }
    let start = page.saturating_mul(DIRECTORY_PAGE_SIZE);
    let end = (start + DIRECTORY_PAGE_SIZE).min(directories.len());
    items.extend(
        directories[start..end]
            .iter()
            .map(|directory| WorkspacePathPickerItem {
                label: format!("› {}/", directory_name(directory)),
                action: Some(WorkspacePathPickerAction::Browse((*directory).clone())),
            }),
    );
    if end < directories.len() {
        items.push(WorkspacePathPickerItem {
            label: "More folders →".to_string(),
            action: Some(WorkspacePathPickerAction::NextPage),
        });
    }
    items
}

fn workspace_path_item_id(index: usize) -> ElementId {
    ElementId::scoped(
        PATH_PICKER_SCOPE,
        FIRST_WORKSPACE_PATH_ITEM.saturating_add(index as u32),
    )
}

fn canonical_directory(path: &Path) -> io::Result<PathBuf> {
    let directory = path.canonicalize()?;
    if directory.is_dir() {
        Ok(directory)
    } else {
        Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("{} is not a directory", directory.display()),
        ))
    }
}

fn read_child_directories(directory: &Path) -> io::Result<Vec<PathBuf>> {
    let mut directories = std::fs::read_dir(directory)?
        .filter_map(Result::ok)
        .filter_map(|entry| {
            entry
                .file_type()
                .ok()
                .filter(|file_type| file_type.is_dir())
                .map(|_| entry.path())
        })
        .collect::<Vec<_>>();
    directories.sort_by(|left, right| {
        directory_name(left)
            .to_lowercase()
            .cmp(&directory_name(right).to_lowercase())
            .then_with(|| left.cmp(right))
    });
    Ok(directories)
}

fn directory_name(directory: &Path) -> String {
    directory
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| directory.display().to_string())
}

fn home_directory() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
}

#[cfg(test)]
#[path = "workspace_path_picker_tests.rs"]
mod tests;
