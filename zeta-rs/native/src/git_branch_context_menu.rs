use zeta_git::GitBranch;
use zeta_ui::{
    ButtonBackgrounds, ButtonState, ButtonStyle, CaretVisibility, Component, ContextMenu,
    ContextMenuItem, ContextMenuSelection, ContextMenuStyle, ContextViewAnchorPosition,
    ContextViewPlacement, CornerRadii, Edges, InputBoxState, Rect, SearchBox, Size, TextInput,
    TextInputCommand, TextInputCompositionEvent, TextInputLayoutEngine, TextStyle, UiScene,
};
use zeta_ui_dispatch::{
    AccessibilityRole, AccessibilitySelection, CursorFeedback, ElementId, FocusBehavior,
    InteractionFrame, NavigationAxis, NavigationGroupId, NodeAction, UiDispatch, UiNode,
};

use crate::shell_interaction::WINDOW;
use crate::shell_style::ShellPalette;

const BRANCH_MENU_SCOPE: u32 = 3;
const GIT_BRANCH_CONTEXT_MENU: ElementId = ElementId::scoped(BRANCH_MENU_SCOPE, 1);
pub(crate) const GIT_BRANCH_SEARCH_INPUT: ElementId = ElementId::scoped(BRANCH_MENU_SCOPE, 2);
const FIRST_GIT_BRANCH_ITEM: u32 = 3;
const BRANCH_PAGE_SIZE: usize = 10;
const MENU_CONTENT_WIDTH: f32 = 260.0;
const MENU_ITEM_HEIGHT: f32 = 30.0;
const MENU_SEARCH_ROW_HEIGHT: f32 = 36.0;
const MENU_SEARCH_INSET: f32 = 4.0;
const MENU_VIEWPORT_MARGIN: f32 = 6.0;
const MENU_ANCHOR_GAP: f32 = 4.0;

#[derive(Clone, Debug, Eq, PartialEq)]
enum GitBranchMenuAction {
    Select(GitBranch),
    PreviousPage,
    NextPage,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct GitBranchMenuItem {
    label: String,
    action: Option<GitBranchMenuAction>,
    current: bool,
}

#[derive(Clone, Debug, PartialEq)]
struct OpenGitBranchContextMenu {
    anchor: Rect,
    branches: Vec<GitBranch>,
    page: usize,
    error: Option<String>,
    restore_focus: Option<ElementId>,
}

/// Product-owned branch list and transient error state for the Git branch menu.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct GitBranchContextMenuState {
    open: Option<OpenGitBranchContextMenu>,
    search_input: TextInput,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum GitBranchMenuActivation {
    PageChanged,
    SelectBranch(GitBranch),
}

impl GitBranchContextMenuState {
    pub(crate) fn open(
        &mut self,
        anchor: Rect,
        mut branches: Vec<GitBranch>,
        restore_focus: Option<ElementId>,
    ) {
        self.search_input.take_text();
        branches.sort_by(|left, right| {
            right
                .is_current()
                .cmp(&left.is_current())
                .then_with(|| left.name().to_lowercase().cmp(&right.name().to_lowercase()))
                .then_with(|| left.name().cmp(right.name()))
        });
        self.open = Some(OpenGitBranchContextMenu {
            anchor,
            branches,
            page: 0,
            error: None,
            restore_focus,
        });
    }

    pub(crate) const fn is_open(&self) -> bool {
        self.open.is_some()
    }

    pub(crate) fn dismiss(&mut self) -> Option<ElementId> {
        self.open.take().and_then(|open| open.restore_focus)
    }

    pub(crate) fn set_switch_error(&mut self) {
        if let Some(open) = self.open.as_mut() {
            open.error = Some("Switch failed · working tree unchanged".to_string());
        }
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
            .find_map(|(index, item)| item.action.as_ref().map(|_| git_branch_menu_item_id(index)))
    }

    pub(crate) fn is_menu_element(&self, id: ElementId) -> bool {
        id == GIT_BRANCH_CONTEXT_MENU || self.item_index(id).is_some()
    }

    pub(crate) fn item_index(&self, id: ElementId) -> Option<usize> {
        self.items()
            .iter()
            .enumerate()
            .find_map(|(index, _)| (git_branch_menu_item_id(index) == id).then_some(index))
    }

    pub(crate) fn activate(&mut self, index: usize) -> Option<GitBranchMenuActivation> {
        let action = self.items().get(index)?.action.clone()?;
        match action {
            GitBranchMenuAction::Select(branch) => {
                Some(GitBranchMenuActivation::SelectBranch(branch))
            }
            GitBranchMenuAction::PreviousPage => {
                if let Some(open) = self.open.as_mut() {
                    open.page = open.page.saturating_sub(1);
                    open.error = None;
                }
                Some(GitBranchMenuActivation::PageChanged)
            }
            GitBranchMenuAction::NextPage => {
                if let Some(open) = self.open.as_mut() {
                    let query = self.search_input.text().trim().to_lowercase();
                    let branch_count = open
                        .branches
                        .iter()
                        .filter(|branch| {
                            query.is_empty() || branch.name().to_lowercase().contains(&query)
                        })
                        .count();
                    let maximum_page = branch_count.saturating_sub(1) / BRANCH_PAGE_SIZE;
                    open.page = (open.page + 1).min(maximum_page);
                    open.error = None;
                }
                Some(GitBranchMenuActivation::PageChanged)
            }
        }
    }

    fn items(&self) -> Vec<GitBranchMenuItem> {
        let Some(open) = self.open.as_ref() else {
            return Vec::new();
        };
        let mut items = Vec::new();
        if let Some(error) = open.error.as_ref() {
            items.push(GitBranchMenuItem {
                label: error.clone(),
                action: None,
                current: false,
            });
        }
        let query = self.search_input.text().trim().to_lowercase();
        let branches = open
            .branches
            .iter()
            .filter(|branch| query.is_empty() || branch.name().to_lowercase().contains(&query))
            .collect::<Vec<_>>();
        if branches.is_empty() {
            items.push(GitBranchMenuItem {
                label: if open.branches.is_empty() {
                    "No local branches".to_string()
                } else {
                    "No matching branches".to_string()
                },
                action: None,
                current: false,
            });
            return items;
        }
        if open.page > 0 {
            items.push(GitBranchMenuItem {
                label: "← Previous branches".to_string(),
                action: Some(GitBranchMenuAction::PreviousPage),
                current: false,
            });
        }
        let start = open.page.saturating_mul(BRANCH_PAGE_SIZE);
        let end = (start + BRANCH_PAGE_SIZE).min(branches.len());
        items.extend(branches[start..end].iter().map(|branch| GitBranchMenuItem {
            label: if branch.is_current() {
                format!("✓ {}", branch.name())
            } else {
                branch.name().to_string()
            },
            action: Some(GitBranchMenuAction::Select((*branch).clone())),
            current: branch.is_current(),
        }));
        if end < branches.len() {
            items.push(GitBranchMenuItem {
                label: "More branches →".to_string(),
                action: Some(GitBranchMenuAction::NextPage),
                current: false,
            });
        }
        items
    }

    fn search_changed(&mut self) {
        if let Some(open) = self.open.as_mut() {
            open.page = 0;
            open.error = None;
        }
    }
}

pub(crate) struct GitBranchContextMenu {
    context_menu: ContextMenu,
    search_box: SearchBox,
    search_value: String,
    items: Vec<GitBranchMenuItem>,
}

impl GitBranchContextMenu {
    pub(crate) fn new(
        viewport: Rect,
        state: &GitBranchContextMenuState,
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
        let menu_items = items
            .iter()
            .enumerate()
            .map(|(index, item)| {
                let id = git_branch_menu_item_id(index);
                let button_state = if item.action.is_none() {
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
                ContextMenuItem::new(item.label.clone(), button_state)
            })
            .collect();
        let selection = items
            .iter()
            .enumerate()
            .find_map(|(index, item)| {
                let id = git_branch_menu_item_id(index);
                (item.action.is_some()
                    && (dispatch.is_pressed(id)
                        || dispatch.is_hovered(id)
                        || dispatch.is_focused(id)))
                .then_some(index)
            })
            .or_else(|| items.iter().position(|item| item.current))
            .map(ContextMenuSelection::Item)
            .unwrap_or(ContextMenuSelection::None);
        let context_menu = ContextMenu::new(
            viewport,
            open.anchor,
            menu_items,
            ContextMenuStyle::new(
                palette.surface,
                button_style,
                Size::new(MENU_CONTENT_WIDTH, MENU_ITEM_HEIGHT),
            )
            .with_header_height(MENU_SEARCH_ROW_HEIGHT)
            .with_placement(
                ContextViewPlacement::new()
                    .with_position(ContextViewAnchorPosition::Before)
                    .with_gap(MENU_ANCHOR_GAP)
                    .with_viewport_margin(MENU_VIEWPORT_MARGIN),
            ),
        )
        .with_selection(selection);
        let header_bounds = context_menu
            .header_bounds()
            .expect("branch menu reserves a search row");
        let search_bounds = Rect::from_xywh(
            header_bounds.origin.x + MENU_SEARCH_INSET,
            header_bounds.origin.y + MENU_SEARCH_INSET,
            (header_bounds.size.width - MENU_SEARCH_INSET * 2.0).max(1.0),
            (header_bounds.size.height - MENU_SEARCH_INSET * 2.0).max(1.0),
        );
        let search_state = if dispatch.is_focused(GIT_BRANCH_SEARCH_INPUT) {
            InputBoxState::Focused(caret_visibility)
        } else if dispatch.is_hovered(GIT_BRANCH_SEARCH_INPUT) {
            InputBoxState::Hovered
        } else {
            InputBoxState::Resting
        };
        let search_box = SearchBox::new(
            search_bounds,
            "Search branches...",
            search_state,
            palette.session_search_style(),
            state.search_input(),
            text_layout,
        );
        Some(Self {
            context_menu,
            search_box,
            search_value: state.search_input().text().to_string(),
            items,
        })
    }

    pub(crate) fn register_interactions(&self, frame: &mut InteractionFrame) {
        frame.register(
            UiNode::new(
                GIT_BRANCH_CONTEXT_MENU,
                self.context_menu.bounds(),
                AccessibilityRole::Menu,
                "Switch Git branch",
            )
            .with_parent(WINDOW),
        );
        frame.set_modal_root(GIT_BRANCH_CONTEXT_MENU);
        let navigation_group = NavigationGroupId::new(GIT_BRANCH_CONTEXT_MENU);
        frame.register(
            UiNode::new(
                GIT_BRANCH_SEARCH_INPUT,
                self.search_box.bounds(),
                AccessibilityRole::TextInput,
                "Search Git branches",
            )
            .with_parent(GIT_BRANCH_CONTEXT_MENU)
            .with_cursor(CursorFeedback::Text)
            .with_focus(FocusBehavior::TabStop)
            .with_navigation(navigation_group, NavigationAxis::Vertical)
            .with_value(&self.search_value),
        );
        for (index, item) in self.items.iter().enumerate() {
            let Some(bounds) = self
                .context_menu
                .interactive_item_bounds(index)
                .filter(|bounds| !bounds.is_empty())
            else {
                continue;
            };
            frame.register(
                UiNode::new(
                    git_branch_menu_item_id(index),
                    bounds,
                    AccessibilityRole::MenuItem,
                    item.label.clone(),
                )
                .with_parent(GIT_BRANCH_CONTEXT_MENU)
                .with_cursor(CursorFeedback::Pointer)
                .with_focus(FocusBehavior::TabStop)
                .with_action(NodeAction::Activate)
                .with_navigation(navigation_group, NavigationAxis::Vertical)
                .with_selection(
                    if self.context_menu.selected_index() == Some(index) {
                        AccessibilitySelection::Selected
                    } else {
                        AccessibilitySelection::Unselected
                    },
                ),
            );
        }
    }

    #[cfg(test)]
    pub(crate) const fn bounds(&self) -> Rect {
        self.context_menu.bounds()
    }

    pub(crate) const fn search_caret_bounds(&self) -> Option<Rect> {
        self.search_box.caret_bounds()
    }
}

impl Component for GitBranchContextMenu {
    fn paint(&self, scene: &mut UiScene) {
        self.context_menu
            .paint_with_header(scene, |scene, _bounds| self.search_box.paint(scene));
    }
}

fn git_branch_menu_item_id(index: usize) -> ElementId {
    ElementId::scoped(
        BRANCH_MENU_SCOPE,
        FIRST_GIT_BRANCH_ITEM.saturating_add(index as u32),
    )
}

#[cfg(test)]
#[path = "git_branch_context_menu_tests.rs"]
mod tests;
