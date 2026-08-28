use zui::ui::ElementId;
use zui::ui::Point;
use zui::ui::Rect;
use zui::ui::TextInput;
use zui::ui::TextInputCommand;
use zui::ui::TextInputCompositionEvent;

use crate::TabGroupId;
use crate::TabInputKey;

#[path = "tab_context_menu/view.rs"]
mod view;

pub use view::TabContextMenu;
pub use view::TabContextMenuStyle;
pub use view::update_tab_context_menu_pointer;

const TAB_CONTEXT_MENU_SCOPE: u32 = 22;
const TAB_CONTEXT_MENU_GROUP_ITEM_SCOPE: u32 = 26;
pub const TAB_CONTEXT_MENU: ElementId = ElementId::scoped(TAB_CONTEXT_MENU_SCOPE, 1);
pub const TAB_CONTEXT_MENU_GROUPS: ElementId = ElementId::scoped(TAB_CONTEXT_MENU_SCOPE, 6);
pub const TAB_RENAME_INPUT: ElementId = ElementId::scoped(TAB_CONTEXT_MENU_SCOPE, 8);
const TAB_CONTEXT_MENU_PIN: ElementId = ElementId::scoped(TAB_CONTEXT_MENU_SCOPE, 2);
const TAB_CONTEXT_MENU_CLOSE: ElementId = ElementId::scoped(TAB_CONTEXT_MENU_SCOPE, 3);
const TAB_CONTEXT_MENU_MOVE_TO_GROUP: ElementId = ElementId::scoped(TAB_CONTEXT_MENU_SCOPE, 4);
const TAB_CONTEXT_MENU_RENAME: ElementId = ElementId::scoped(TAB_CONTEXT_MENU_SCOPE, 5);
pub const TAB_CONTEXT_MENU_MOVE_TO_NEW_GROUP: ElementId =
    ElementId::scoped(TAB_CONTEXT_MENU_SCOPE, 7);

/// Root action emitted by the Workbench tab context menu.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TabContextMenuAction {
    TogglePin,
    Close,
    MoveToGroup,
    Rename,
}

impl TabContextMenuAction {
    pub const ALL: [Self; 4] = [
        Self::TogglePin,
        Self::Close,
        Self::MoveToGroup,
        Self::Rename,
    ];

    pub const fn element_id(self) -> ElementId {
        match self {
            Self::TogglePin => TAB_CONTEXT_MENU_PIN,
            Self::Close => TAB_CONTEXT_MENU_CLOSE,
            Self::MoveToGroup => TAB_CONTEXT_MENU_MOVE_TO_GROUP,
            Self::Rename => TAB_CONTEXT_MENU_RENAME,
        }
    }

    pub const fn label(self, pinned: bool) -> &'static str {
        match self {
            Self::TogglePin if pinned => "Unpin tab",
            Self::TogglePin => "Pin tab",
            Self::Close => "Close tab",
            Self::MoveToGroup => "Move to group  ›",
            Self::Rename => "Rename tab",
        }
    }

    pub const fn from_element_id(id: ElementId) -> Option<Self> {
        match id {
            TAB_CONTEXT_MENU_PIN => Some(Self::TogglePin),
            TAB_CONTEXT_MENU_CLOSE => Some(Self::Close),
            TAB_CONTEXT_MENU_MOVE_TO_GROUP => Some(Self::MoveToGroup),
            TAB_CONTEXT_MENU_RENAME => Some(Self::Rename),
            _ => None,
        }
    }

    pub fn is_menu_element(id: ElementId) -> bool {
        id == TAB_CONTEXT_MENU
            || id == TAB_CONTEXT_MENU_GROUPS
            || id == TAB_RENAME_INPUT
            || id == TAB_CONTEXT_MENU_MOVE_TO_NEW_GROUP
            || Self::from_element_id(id).is_some()
            || tab_group_for_menu_element(id).is_some()
    }
}

/// Result requested after one menu item is activated.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TabContextMenuActivation {
    Ignored,
    OpenGroupMenu,
    TogglePin(TabInputKey),
    Close(TabInputKey),
    MoveToGroup(TabInputKey, TabGroupId),
    MoveToNewGroup(TabInputKey),
    BeginRename(TabInputKey),
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
enum TabContextMenuView {
    #[default]
    Actions,
    Groups,
    Rename,
}

#[derive(Clone, Debug, PartialEq)]
struct OpenTabContextMenu {
    target_tab: TabInputKey,
    anchor: Rect,
    restore_focus: Option<ElementId>,
    pinned: bool,
    view: TabContextMenuView,
    rename: TextInput,
}

/// Workbench-owned transient state for tab actions, group selection, and tab-name editing.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TabContextMenuState {
    open: Option<OpenTabContextMenu>,
}

impl TabContextMenuState {
    pub fn open_pinned(
        &mut self,
        target_tab: TabInputKey,
        position: Point,
        restore_focus: Option<ElementId>,
    ) {
        self.open(target_tab, position, restore_focus, true);
    }

    pub fn open_unpinned(
        &mut self,
        target_tab: TabInputKey,
        position: Point,
        restore_focus: Option<ElementId>,
    ) {
        self.open(target_tab, position, restore_focus, false);
    }

    fn open(
        &mut self,
        target_tab: TabInputKey,
        position: Point,
        restore_focus: Option<ElementId>,
        pinned: bool,
    ) {
        self.open = Some(OpenTabContextMenu {
            target_tab,
            anchor: Rect::from_xywh(position.x, position.y, 1.0, 1.0),
            restore_focus,
            pinned,
            view: TabContextMenuView::Actions,
            rename: TextInput::new(),
        });
    }

    pub const fn is_open(&self) -> bool {
        self.open.is_some()
    }

    pub fn is_renaming(&self) -> bool {
        self.open
            .as_ref()
            .is_some_and(|open| open.view == TabContextMenuView::Rename)
    }

    pub fn dismiss(&mut self) -> Option<ElementId> {
        self.open.take().and_then(|open| open.restore_focus)
    }

    pub fn target_tab(&self) -> Option<&TabInputKey> {
        self.open.as_ref().map(|open| &open.target_tab)
    }

    #[cfg(test)]
    pub fn target_is_pinned(&self) -> bool {
        self.open.as_ref().is_some_and(|open| open.pinned)
    }

    pub fn activate(&mut self, id: ElementId) -> TabContextMenuActivation {
        let Some(open) = self.open.as_mut() else {
            return TabContextMenuActivation::Ignored;
        };
        if let Some(group) = tab_group_for_menu_element(id) {
            return TabContextMenuActivation::MoveToGroup(open.target_tab.clone(), group);
        }
        if id == TAB_CONTEXT_MENU_MOVE_TO_NEW_GROUP {
            return TabContextMenuActivation::MoveToNewGroup(open.target_tab.clone());
        }
        match TabContextMenuAction::from_element_id(id) {
            Some(TabContextMenuAction::TogglePin) => {
                TabContextMenuActivation::TogglePin(open.target_tab.clone())
            }
            Some(TabContextMenuAction::Close) => {
                TabContextMenuActivation::Close(open.target_tab.clone())
            }
            Some(TabContextMenuAction::MoveToGroup) => {
                open.view = TabContextMenuView::Groups;
                TabContextMenuActivation::OpenGroupMenu
            }
            Some(TabContextMenuAction::Rename) => {
                open.view = TabContextMenuView::Rename;
                TabContextMenuActivation::BeginRename(open.target_tab.clone())
            }
            None => TabContextMenuActivation::Ignored,
        }
    }

    pub fn set_rename_text(&mut self, title: &str) -> bool {
        let Some(open) = self.open.as_mut() else {
            return false;
        };
        open.rename = TextInput::new();
        open.rename
            .apply(TextInputCommand::Insert(title.to_owned()));
        open.rename.apply(TextInputCommand::SelectAll);
        true
    }

    pub fn apply_rename(&mut self, command: TextInputCommand) -> bool {
        let Some(open) = self
            .open
            .as_mut()
            .filter(|open| open.view == TabContextMenuView::Rename)
        else {
            return false;
        };
        open.rename.apply(command);
        true
    }

    pub fn apply_rename_composition(&mut self, event: TextInputCompositionEvent) -> bool {
        let Some(open) = self
            .open
            .as_mut()
            .filter(|open| open.view == TabContextMenuView::Rename)
        else {
            return false;
        };
        open.rename.apply_composition(event);
        true
    }

    pub fn take_rename(&self) -> Option<(TabInputKey, String)> {
        let open = self
            .open
            .as_ref()
            .filter(|open| open.view == TabContextMenuView::Rename)?;
        let title = open.rename.text().trim();
        (!title.is_empty()).then(|| (open.target_tab.clone(), title.to_owned()))
    }
}

pub fn tab_group_menu_element_id(group: TabGroupId) -> ElementId {
    let local = u32::try_from(group.value()).expect("tab group identity must fit menu scope");
    ElementId::scoped(TAB_CONTEXT_MENU_GROUP_ITEM_SCOPE, local)
}

pub fn tab_group_for_menu_element(id: ElementId) -> Option<TabGroupId> {
    let raw = id.into_raw();
    ((raw >> 32) == u64::from(TAB_CONTEXT_MENU_GROUP_ITEM_SCOPE))
        .then(|| TabGroupId::from_value(raw & u64::from(u32::MAX)))
}

#[cfg(test)]
#[path = "tab_context_menu_tests.rs"]
mod tests;
