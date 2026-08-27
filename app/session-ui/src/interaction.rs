//! Stable interaction identities owned by the Session UI feature.

use zui::ui::ElementId;

pub const SESSION_HEADER: ElementId = ElementId::scoped(16, 1);
pub const SESSION_CONTEXT_MENU: ElementId = ElementId::scoped(1, 17);

const SESSION_CONTEXT_MENU_PIN: ElementId = ElementId::scoped(1, 18);
const SESSION_CONTEXT_MENU_CLOSE: ElementId = ElementId::scoped(1, 19);
const SESSION_CONTEXT_MENU_RENAME: ElementId = ElementId::scoped(1, 20);
const SESSION_CONTEXT_MENU_FORK: ElementId = ElementId::scoped(1, 21);

/// Action emitted by the Session tab context menu.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SessionContextMenuAction {
    Pin,
    Close,
    Rename,
    Fork,
}

impl SessionContextMenuAction {
    pub const ALL: [Self; 4] = [Self::Pin, Self::Close, Self::Rename, Self::Fork];

    pub const fn element_id(self) -> ElementId {
        match self {
            Self::Pin => SESSION_CONTEXT_MENU_PIN,
            Self::Close => SESSION_CONTEXT_MENU_CLOSE,
            Self::Rename => SESSION_CONTEXT_MENU_RENAME,
            Self::Fork => SESSION_CONTEXT_MENU_FORK,
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::Pin => "Pin",
            Self::Close => "Close",
            Self::Rename => "Rename",
            Self::Fork => "Fork",
        }
    }

    pub const fn from_element_id(id: ElementId) -> Option<Self> {
        match id {
            SESSION_CONTEXT_MENU_PIN => Some(Self::Pin),
            SESSION_CONTEXT_MENU_CLOSE => Some(Self::Close),
            SESSION_CONTEXT_MENU_RENAME => Some(Self::Rename),
            SESSION_CONTEXT_MENU_FORK => Some(Self::Fork),
            _ => None,
        }
    }

    pub fn is_menu_element(id: ElementId) -> bool {
        id == SESSION_CONTEXT_MENU || Self::from_element_id(id).is_some()
    }
}
