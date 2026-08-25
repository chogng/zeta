use std::cell::RefCell;
use std::error::Error;
use std::fmt;
use std::rc::Rc;
use std::sync::Arc;

use super::SystemServiceError;

#[cfg(target_os = "windows")]
mod windows;

/// Stable application-owned identity for one actionable native menu item.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct MenuItemId(String);

impl MenuItemId {
    /// Creates a non-empty application-owned menu identity.
    pub fn new(value: impl Into<String>) -> Result<Self, MenuItemIdError> {
        let value = value.into();
        if value.trim().is_empty() {
            return Err(MenuItemIdError);
        }
        Ok(Self(value))
    }

    /// Returns the stable string identity.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    #[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows"))]
    fn from_native(value: &str) -> Self {
        Self(value.to_owned())
    }
}

/// Failure to create an empty menu item identity.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MenuItemIdError;

impl fmt::Display for MenuItemIdError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("menu item identity cannot be empty")
    }
}

impl Error for MenuItemIdError {}

/// One actionable item in a backend-independent menu model.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MenuAction {
    pub id: MenuItemId,
    pub label: String,
    pub enabled: bool,
}

impl MenuAction {
    /// Creates an enabled action.
    pub fn new(id: MenuItemId, label: impl Into<String>) -> Self {
        Self {
            id,
            label: label.into(),
            enabled: true,
        }
    }

    /// Sets whether the action can currently be selected.
    pub const fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }
}

/// Action, separator, or nested submenu in a menu group.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MenuEntry {
    Action(MenuAction),
    Separator,
    Submenu(MenuGroup),
}

/// Labeled menu containing actions and nested groups.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MenuGroup {
    pub id: MenuItemId,
    pub label: String,
    pub enabled: bool,
    pub entries: Vec<MenuEntry>,
}

impl MenuGroup {
    /// Creates an enabled menu group.
    pub fn new(
        id: MenuItemId,
        label: impl Into<String>,
        entries: impl IntoIterator<Item = MenuEntry>,
    ) -> Self {
        Self {
            id,
            label: label.into(),
            enabled: true,
            entries: entries.into_iter().collect(),
        }
    }
}

/// Complete backend-independent application-menu model.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct MenuModel {
    pub groups: Vec<MenuGroup>,
}

impl MenuModel {
    /// Creates a menu model from top-level groups.
    pub fn new(groups: impl IntoIterator<Item = MenuGroup>) -> Self {
        Self {
            groups: groups.into_iter().collect(),
        }
    }
}

/// Thread-safe callback installed by the runtime to receive native menu actions.
pub type MenuEventHandler = Arc<dyn Fn(MenuItemId) + Send + Sync>;

/// Main-thread native menu backend used through an injectable [`MenuHandle`].
pub trait MenuService {
    /// Replaces the current application menu.
    fn set_application_menu(&mut self, model: MenuModel) -> Result<(), SystemServiceError>;

    /// Installs or clears the runtime event callback.
    fn set_event_handler(&mut self, handler: Option<MenuEventHandler>);

    /// Attaches the current application menu to a newly opened window on platforms with
    /// per-window native menu ownership.
    fn attach_window(
        &mut self,
        _window: crate::window::WindowHandle,
    ) -> Result<(), SystemServiceError> {
        Ok(())
    }

    /// Detaches native menu resources before a runtime-owned window is destroyed.
    fn detach_window(&mut self, _window: crate::window::WindowId) {}
}

/// Cloneable main-thread capability for configuring the application menu.
#[derive(Clone)]
pub struct MenuHandle {
    service: Rc<RefCell<Box<dyn MenuService>>>,
}

impl MenuHandle {
    pub(crate) fn new(service: impl MenuService + 'static) -> Self {
        Self {
            service: Rc::new(RefCell::new(Box::new(service))),
        }
    }

    /// Replaces the application menu through the injected backend.
    pub fn set_application_menu(&self, model: MenuModel) -> Result<(), SystemServiceError> {
        self.service.borrow_mut().set_application_menu(model)
    }

    pub(crate) fn set_event_handler(&self, handler: Option<MenuEventHandler>) {
        self.service.borrow_mut().set_event_handler(handler);
    }

    pub(crate) fn attach_window(
        &self,
        window: crate::window::WindowHandle,
    ) -> Result<(), SystemServiceError> {
        self.service.borrow_mut().attach_window(window)
    }

    pub(crate) fn detach_window(&self, window: crate::window::WindowId) {
        self.service.borrow_mut().detach_window(window);
    }
}

/// Default native application-menu backend.
#[derive(Default)]
pub struct SystemMenu {
    #[cfg(target_os = "macos")]
    menu: Option<muda::Menu>,
    #[cfg(target_os = "windows")]
    menu: Option<muda::Menu>,
    #[cfg(target_os = "windows")]
    windows: std::collections::HashMap<crate::window::WindowId, isize>,
}

impl MenuService for SystemMenu {
    fn set_application_menu(&mut self, model: MenuModel) -> Result<(), SystemServiceError> {
        #[cfg(target_os = "macos")]
        {
            let menu = build_native_menu(model)?;
            menu.init_for_nsapp();
            self.menu = Some(menu);
            Ok(())
        }
        #[cfg(target_os = "windows")]
        {
            if let Some(previous) = self.menu.take() {
                for hwnd in self.windows.values().copied() {
                    let _ = windows::remove(&previous, hwnd);
                }
            }
            let menu = build_native_menu(model)?;
            let mut attached = Vec::new();
            for hwnd in self.windows.values().copied() {
                if let Err(source) = windows::attach(&menu, hwnd) {
                    for hwnd in attached {
                        let _ = windows::remove(&menu, hwnd);
                    }
                    return Err(source);
                }
                attached.push(hwnd);
            }
            self.menu = Some(menu);
            Ok(())
        }
        #[cfg(not(any(target_os = "macos", target_os = "windows")))]
        {
            let _ = model;
            Err(SystemServiceError::unsupported("native application menu"))
        }
    }

    fn set_event_handler(&mut self, handler: Option<MenuEventHandler>) {
        #[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows"))]
        {
            muda::MenuEvent::set_event_handler(handler.map(|handler| {
                move |event: muda::MenuEvent| {
                    handler(MenuItemId::from_native(event.id.as_ref()));
                }
            }));
        }
        #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
        let _ = handler;
    }

    fn attach_window(
        &mut self,
        window: crate::window::WindowHandle,
    ) -> Result<(), SystemServiceError> {
        #[cfg(target_os = "windows")]
        {
            let id = window.id().ok_or_else(|| {
                SystemServiceError::backend(
                    "native application menu",
                    std::io::Error::other("menu target window is no longer live"),
                )
            })?;
            if self.windows.contains_key(&id) {
                return Ok(());
            }
            let hwnd = window.native_hwnd().ok_or_else(|| {
                SystemServiceError::backend(
                    "native application menu",
                    std::io::Error::other("window does not expose a Win32 HWND"),
                )
            })?;
            if let Some(menu) = &self.menu {
                windows::attach(menu, hwnd)?;
            }
            self.windows.insert(id, hwnd);
        }
        #[cfg(not(target_os = "windows"))]
        let _ = window;
        Ok(())
    }

    fn detach_window(&mut self, window: crate::window::WindowId) {
        #[cfg(target_os = "windows")]
        if let Some(hwnd) = self.windows.remove(&window)
            && let Some(menu) = &self.menu
        {
            let _ = windows::remove(menu, hwnd);
        }
        #[cfg(not(target_os = "windows"))]
        let _ = window;
    }
}

#[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows"))]
pub(super) fn build_native_menu(model: MenuModel) -> Result<muda::Menu, SystemServiceError> {
    let menu = muda::Menu::new();
    for group in model.groups {
        let submenu = build_submenu(group)?;
        menu.append(&submenu)
            .map_err(|source| SystemServiceError::backend("native application menu", source))?;
    }
    Ok(menu)
}

#[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows"))]
fn build_submenu(group: MenuGroup) -> Result<muda::Submenu, SystemServiceError> {
    let submenu = muda::Submenu::with_id(group.id.as_str(), group.label, group.enabled);
    for entry in group.entries {
        match entry {
            MenuEntry::Action(action) => {
                let item =
                    muda::MenuItem::with_id(action.id.as_str(), action.label, action.enabled, None);
                submenu.append(&item).map_err(|source| {
                    SystemServiceError::backend("native application menu", source)
                })?;
            }
            MenuEntry::Separator => {
                let separator = muda::PredefinedMenuItem::separator();
                submenu.append(&separator).map_err(|source| {
                    SystemServiceError::backend("native application menu", source)
                })?;
            }
            MenuEntry::Submenu(group) => {
                let child = build_submenu(group)?;
                submenu.append(&child).map_err(|source| {
                    SystemServiceError::backend("native application menu", source)
                })?;
            }
        }
    }
    Ok(submenu)
}
