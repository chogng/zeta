//! Typed operating-system capabilities owned and injected by the application runtime.

mod clipboard;
mod error;
mod file_dialog;
mod global_shortcut;
mod menu;
mod notification;
mod opener;
mod process;
mod resource;
mod tray;
mod update;

pub use clipboard::Clipboard;
pub use clipboard::ClipboardError;
pub use clipboard::ClipboardHandle;
pub use clipboard::SystemClipboard;
pub use error::SystemServiceError;
pub use file_dialog::FileDialogFilter;
pub use file_dialog::FileDialogHandle;
pub use file_dialog::FileDialogOptions;
pub use file_dialog::FileDialogService;
pub use file_dialog::SystemFileDialogs;
pub use global_shortcut::GlobalShortcut;
pub use global_shortcut::GlobalShortcutEvent;
pub use global_shortcut::GlobalShortcutEventHandler;
pub use global_shortcut::GlobalShortcutHandle;
pub use global_shortcut::GlobalShortcutId;
pub use global_shortcut::GlobalShortcutIdError;
pub use global_shortcut::GlobalShortcutService;
pub use global_shortcut::GlobalShortcutState;
pub use global_shortcut::ShortcutAccelerator;
pub use global_shortcut::ShortcutAcceleratorError;
pub use global_shortcut::SystemGlobalShortcuts;
pub use menu::MenuAction;
pub use menu::MenuEntry;
pub use menu::MenuEventHandler;
pub use menu::MenuGroup;
pub use menu::MenuHandle;
pub use menu::MenuItemId;
pub use menu::MenuItemIdError;
pub use menu::MenuModel;
pub use menu::MenuService;
pub use menu::SystemMenu;
pub use notification::NotificationHandle;
pub use notification::NotificationId;
pub use notification::NotificationRequest;
pub use notification::NotificationService;
pub use notification::SystemNotifications;
pub use opener::ExternalUrl;
pub use opener::ExternalUrlError;
pub use opener::OpenTarget;
pub use opener::OpenerHandle;
pub use opener::OpenerService;
pub use opener::SystemOpener;
pub use process::ChildProcess;
pub use process::PlatformProcessSandbox;
pub use process::PreparedProcessCommand;
pub use process::ProcessCommand;
pub use process::ProcessController;
pub use process::ProcessDropPolicy;
pub use process::ProcessEnvironment;
pub use process::ProcessExit;
pub use process::ProcessFileSystemAccess;
pub use process::ProcessHandle;
pub use process::ProcessId;
pub use process::ProcessNetworkAccess;
pub use process::ProcessSandbox;
pub use process::ProcessSandboxError;
pub use process::ProcessSandboxKind;
pub use process::ProcessSandboxPolicy;
pub use process::ProcessService;
pub use process::ProcessStdio;
pub use process::SystemProcesses;
#[doc(hidden)]
pub use process::appcontainer_runner_main;
pub use resource::ResourceHandle;
pub use resource::ResourcePath;
pub use resource::ResourcePathError;
pub use resource::ResourceService;
pub use resource::SystemResourceLocator;
pub use tray::SystemTray;
pub use tray::TrayBounds;
pub use tray::TrayButtonState;
pub use tray::TrayEvent;
pub use tray::TrayEventHandler;
pub use tray::TrayEventKind;
pub use tray::TrayHandle;
pub use tray::TrayIconImage;
pub use tray::TrayIconImageError;
pub use tray::TrayId;
pub use tray::TrayIdError;
pub use tray::TrayMouseButton;
pub use tray::TrayOptions;
pub use tray::TrayPosition;
pub use tray::TrayService;
pub use update::AppVersion;
pub use update::AppVersionError;
pub use update::DisabledUpdates;
pub use update::HttpUpdateTransport;
pub use update::SignedHttpUpdater;
pub use update::StagedUpdate;
pub use update::SystemUpdateInstaller;
pub use update::UpdateArtifact;
pub use update::UpdateConfig;
pub use update::UpdateHandle;
pub use update::UpdateInstaller;
pub use update::UpdatePublicKey;
pub use update::UpdateRelease;
pub use update::UpdateService;
pub use update::UpdateTransport;

/// Cloneable typed collection of operating-system services available to an application.
#[derive(Clone)]
pub struct Services {
    file_dialogs: FileDialogHandle,
    opener: OpenerHandle,
    notifications: NotificationHandle,
    menus: MenuHandle,
    tray: TrayHandle,
    global_shortcuts: GlobalShortcutHandle,
    resources: ResourceHandle,
    processes: ProcessHandle,
    updates: UpdateHandle,
}

impl Services {
    pub(super) fn system() -> Self {
        Self {
            file_dialogs: FileDialogHandle::new(SystemFileDialogs),
            opener: OpenerHandle::new(SystemOpener),
            notifications: NotificationHandle::new(SystemNotifications),
            menus: MenuHandle::new(SystemMenu::default()),
            tray: TrayHandle::new(SystemTray::default()),
            global_shortcuts: GlobalShortcutHandle::new(SystemGlobalShortcuts::default()),
            resources: ResourceHandle::new(SystemResourceLocator::default()),
            processes: ProcessHandle::new(SystemProcesses::default()),
            updates: UpdateHandle::new(DisabledUpdates),
        }
    }

    /// Returns the injected file-dialog capability.
    pub fn file_dialogs(&self) -> FileDialogHandle {
        self.file_dialogs.clone()
    }

    /// Returns the injected system opener capability.
    pub fn opener(&self) -> OpenerHandle {
        self.opener.clone()
    }

    /// Returns the injected desktop-notification capability.
    pub fn notifications(&self) -> NotificationHandle {
        self.notifications.clone()
    }

    /// Returns the main-thread application-menu capability.
    pub fn menus(&self) -> MenuHandle {
        self.menus.clone()
    }

    /// Returns the main-thread system-tray capability.
    pub fn tray(&self) -> TrayHandle {
        self.tray.clone()
    }

    /// Returns the main-thread global-shortcut capability.
    pub fn global_shortcuts(&self) -> GlobalShortcutHandle {
        self.global_shortcuts.clone()
    }

    /// Returns the application resource-location capability.
    pub fn resources(&self) -> ResourceHandle {
        self.resources.clone()
    }

    /// Returns the shell-free managed child-process capability.
    pub fn processes(&self) -> ProcessHandle {
        self.processes.clone()
    }

    /// Returns the application update capability.
    pub fn updates(&self) -> UpdateHandle {
        self.updates.clone()
    }

    pub(super) fn replace_file_dialogs(&mut self, service: impl FileDialogService + 'static) {
        self.file_dialogs = FileDialogHandle::new(service);
    }

    pub(super) fn replace_opener(&mut self, service: impl OpenerService + 'static) {
        self.opener = OpenerHandle::new(service);
    }

    pub(super) fn replace_notifications(&mut self, service: impl NotificationService + 'static) {
        self.notifications = NotificationHandle::new(service);
    }

    pub(super) fn replace_menus(&mut self, service: impl MenuService + 'static) {
        self.menus = MenuHandle::new(service);
    }

    pub(super) fn replace_tray(&mut self, service: impl TrayService + 'static) {
        self.tray = TrayHandle::new(service);
    }

    pub(super) fn replace_global_shortcuts(
        &mut self,
        service: impl GlobalShortcutService + 'static,
    ) {
        self.global_shortcuts = GlobalShortcutHandle::new(service);
    }

    pub(super) fn replace_resources(&mut self, service: impl ResourceService + 'static) {
        self.resources = ResourceHandle::new(service);
    }

    pub(super) fn replace_processes(&mut self, service: impl ProcessService + 'static) {
        self.processes = ProcessHandle::new(service);
    }

    pub(super) fn replace_updates(&mut self, service: impl UpdateService + 'static) {
        self.updates = UpdateHandle::new(service);
    }
}

#[cfg(test)]
#[path = "services/services_tests.rs"]
mod tests;
