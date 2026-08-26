use std::path::PathBuf;
use std::process::ExitCode;
use std::time::Duration;

use zui::app::App;
use zui::app::AppContext;
use zui::app::Application;
use zui::app::ApplicationActivation;
use zui::app::ApplicationError;
use zui::app::ApplicationExitDecision;
use zui::app::ApplicationExitReason;
use zui::app::ApplicationFocusOptions;
use zui::app::ApplicationHandle;
use zui::app::ApplicationLocale;
use zui::app::ApplicationPath;
use zui::app::ApplicationReadyError;
use zui::app::OpenWindowError;
use zui::app::ProtocolScheme;
use zui::app::ProtocolUrl;
use zui::app::SecondInstance;
use zui::app::SingleInstanceKey;
use zui::app::SingleInstanceOptions;
use zui::app::SingleInstanceRun;
use zui::app::WindowContext;
use zui::services::ApplicationBadgeRequest;
use zui::services::ApplicationBadgeService;
use zui::services::DesktopFileName;
use zui::services::FileDialogOptions;
use zui::services::FileIconImage;
use zui::services::FileIconRequest;
use zui::services::FileIconService;
use zui::services::FileIconSize;
use zui::services::GlobalShortcut;
use zui::services::GlobalShortcutEvent;
use zui::services::GlobalShortcutId;
use zui::services::JumpListRequest;
use zui::services::JumpListService;
use zui::services::JumpListSettings;
use zui::services::JumpListTask;
use zui::services::JumpListUpdateResult;
use zui::services::LoginItemName;
use zui::services::LoginItemOptions;
use zui::services::LoginItemRequest;
use zui::services::LoginItemService;
use zui::services::LoginItemSettings;
use zui::services::LoginItemState;
use zui::services::LoginItemStatus;
use zui::services::LoginItemUpdate;
use zui::services::MenuAccelerator;
use zui::services::MenuAction;
use zui::services::MenuEntry;
use zui::services::MenuGroup;
use zui::services::MenuItemId;
use zui::services::MenuModel;
use zui::services::MenuRole;
use zui::services::MenuRoleItem;
use zui::services::MessageDialogButtons;
use zui::services::MessageDialogRequest;
use zui::services::MessageDialogResponse;
use zui::services::ProtocolClientOptions;
use zui::services::ProtocolClientRemoval;
use zui::services::ProtocolClientRequest;
use zui::services::ProtocolClientService;
use zui::services::ShortcutAccelerator;
use zui::services::SystemServiceError;
use zui::services::TrayEvent;
use zui::services::TrayEventKind;
use zui::services::TrayIconImage;
use zui::services::TrayId;
use zui::services::TrayOptions;
use zui::ui::ComponentRuntime;
use zui::ui::ViewState;
use zui::window::DisplayEvent;
use zui::window::LogicalPosition;
use zui::window::LogicalSize;
use zui::window::OpenedWindow;
use zui::window::PhysicalPosition;
use zui::window::WindowChrome;
use zui::window::WindowEvent;
use zui::window::WindowHandle;
use zui::window::WindowId;
use zui::window::WindowLevel;
use zui::window::WindowOptions;

enum DemoEvent {
    BackgroundReady,
    FrameworkReady(Result<(), ApplicationReadyError>),
    FileIconLoaded(Result<FileIconImage, SystemServiceError>),
    FileSelected(Result<Option<PathBuf>, SystemServiceError>),
    MessageAnswered(Result<MessageDialogResponse, SystemServiceError>),
    RenameWindow(WindowId),
    WindowOpened(Result<OpenedWindow, OpenWindowError>),
}

struct DemoApp {
    application: ApplicationHandle<DemoEvent>,
    windows: Vec<DemoWindow>,
    background_ready: ViewState<bool>,
    display_events: usize,
}

struct DemoWindow {
    handle: WindowHandle,
    components: ComponentRuntime,
}

#[derive(Clone, Copy)]
struct DemoSystemIntegration;

impl ProtocolClientService for DemoSystemIntegration {
    fn set_default(&mut self, _request: &ProtocolClientRequest) -> Result<(), SystemServiceError> {
        Ok(())
    }

    fn is_default(&mut self, _request: &ProtocolClientRequest) -> Result<bool, SystemServiceError> {
        Ok(true)
    }

    fn remove_default(
        &mut self,
        _request: &ProtocolClientRequest,
    ) -> Result<ProtocolClientRemoval, SystemServiceError> {
        Ok(ProtocolClientRemoval::Removed)
    }
}

impl LoginItemService for DemoSystemIntegration {
    fn set(&mut self, _update: &LoginItemUpdate) -> Result<(), SystemServiceError> {
        Ok(())
    }

    fn get(&mut self, _request: &LoginItemRequest) -> Result<LoginItemState, SystemServiceError> {
        Ok(LoginItemState::new(LoginItemStatus::Enabled))
    }
}

impl ApplicationBadgeService for DemoSystemIntegration {
    fn set(&mut self, request: &ApplicationBadgeRequest) -> Result<(), SystemServiceError> {
        debug_assert_eq!(
            request.desktop_file_name().map(DesktopFileName::as_str),
            Some("dev.zeta.zui-demo.desktop")
        );
        Ok(())
    }
}

impl JumpListService for DemoSystemIntegration {
    fn settings(&mut self) -> Result<JumpListSettings, SystemServiceError> {
        Ok(JumpListSettings::new(10, Vec::new()))
    }

    fn set(
        &mut self,
        _request: &JumpListRequest,
    ) -> Result<JumpListUpdateResult, SystemServiceError> {
        Ok(JumpListUpdateResult::Applied)
    }
}

impl FileIconService for DemoSystemIntegration {
    fn load(&self, request: &FileIconRequest) -> Result<FileIconImage, SystemServiceError> {
        let size = match request.size() {
            FileIconSize::Small => 16,
            FileIconSize::Normal | FileIconSize::Large => 32,
        };
        FileIconImage::from_rgba(
            [0x2f, 0x81, 0xf7, 0xff].repeat(size * size),
            size as u32,
            size as u32,
        )
        .map_err(|source| SystemServiceError::backend("file icon", source))
    }
}

impl DemoApp {
    fn new(application: ApplicationHandle<DemoEvent>) -> Self {
        debug_assert!(!application.is_ready());
        let ready = application.when_ready();
        application
            .spawn(async move { DemoEvent::FrameworkReady(ready.await) })
            .detach();
        Self {
            application,
            windows: Vec::new(),
            background_ready: ViewState::new(false),
            display_events: 0,
        }
    }

    fn track_window(&mut self, handle: WindowHandle) {
        let redraw = handle.clone();
        self.windows.push(DemoWindow {
            handle,
            components: ComponentRuntime::new(move |_| {
                let _ = redraw.request_redraw();
            }),
        });
    }

    fn mark_ready(&self) {
        self.background_ready.update(|ready| *ready = true);
    }
}

impl App<DemoEvent> for DemoApp {
    fn ready(&mut self, context: &mut AppContext<'_, DemoEvent>) {
        if !self.windows.is_empty() {
            return;
        }
        debug_assert_eq!(
            context.application_name().to_string_lossy(),
            "ZUI Native Demo"
        );
        debug_assert_eq!(context.application_version(), "1.0.0-demo.1");
        debug_assert_eq!(context.application_locale().as_str(), "en-US");
        debug_assert!(
            context
                .preferred_system_languages()
                .iter()
                .all(|locale| ApplicationLocale::new(locale.as_str()).is_ok())
        );
        let executable = context
            .path(ApplicationPath::Executable)
            .expect("native demo executable path");
        debug_assert_eq!(context.application_path(), executable.parent().unwrap());
        debug_assert_eq!(
            context.path(ApplicationPath::SessionData).unwrap(),
            context.path(ApplicationPath::Temporary).unwrap()
        );
        let protocol_scheme = ProtocolScheme::new("zui-demo").unwrap();
        let protocol_options = ProtocolClientOptions::new();
        context
            .set_as_default_protocol_client_with(protocol_scheme.clone(), protocol_options.clone())
            .expect("injected protocol-client registration");
        let is_default = context
            .is_default_protocol_client_with(protocol_scheme.clone(), protocol_options.clone())
            .expect("injected protocol-client lookup");
        let removal = context
            .remove_as_default_protocol_client_with(protocol_scheme, protocol_options)
            .expect("injected protocol-client removal");
        debug_assert!(is_default);
        debug_assert_eq!(removal, ProtocolClientRemoval::Removed);
        let login_options = LoginItemOptions::new()
            .with_executable(executable.clone())
            .with_name(LoginItemName::new("ZUI Native Demo").unwrap());
        context
            .set_login_item_settings(LoginItemSettings::enable(login_options.clone()))
            .expect("injected login-item update");
        let login_state = context
            .login_item_settings(login_options)
            .expect("injected login-item lookup");
        debug_assert_eq!(login_state.status(), LoginItemStatus::Enabled);
        context
            .set_badge_count(7)
            .expect("injected application badge update");
        debug_assert_eq!(context.badge_count(), 7);
        context
            .clear_application_badge()
            .expect("injected application badge clear");
        let task = JumpListTask::new(executable.clone(), "New demo window")
            .with_arguments("--new-window")
            .with_description("Open another ZUI native demo window");
        debug_assert_eq!(
            context
                .set_user_tasks(vec![task])
                .expect("injected Jump List task update"),
            JumpListUpdateResult::Applied
        );
        debug_assert_eq!(
            context
                .jump_list_settings()
                .expect("injected Jump List settings")
                .min_items(),
            10
        );
        context.reset_jump_list().expect("injected Jump List reset");
        let icon = context.get_file_icon(executable.clone());
        context
            .spawn(async move { DemoEvent::FileIconLoaded(icon.await) })
            .detach();
        for (index, title) in ["zui app demo", "zui app demo · second window"]
            .into_iter()
            .enumerate()
        {
            let options = WindowOptions::new(title)
                .with_inner_size(LogicalSize::new(480.0, 240.0))
                .with_resize_increments(LogicalSize::new(8.0, 8.0))
                .with_position(LogicalPosition::new(
                    80.0 + index as f64 * 40.0,
                    80.0 + index as f64 * 40.0,
                ))
                .with_window_level(if index == 0 {
                    WindowLevel::Normal
                } else {
                    WindowLevel::AlwaysOnTop
                })
                .with_chrome(WindowChrome::Native);
            #[cfg(any(target_os = "macos", target_os = "windows"))]
            let options = if index == 1 {
                options.with_parent(self.windows[0].handle.id())
            } else {
                options
            };
            match context.open_window(options) {
                Ok(opened) => {
                    let window = opened.handle();
                    if let Err(error) = window.request_redraw() {
                        context.exit_with_error(ApplicationError::product(
                            "initial demo redraw",
                            error,
                        ));
                        return;
                    }
                    self.track_window(window);
                }
                Err(error) => {
                    context.exit_with_error(error);
                    return;
                }
            }
        }
        let displays = context.display_snapshot();
        if displays.displays().is_empty() {
            eprintln!("the platform did not report any connected displays");
        }
        match context.cursor_screen_position() {
            Ok(position) => debug_assert!(displays.display_nearest_point(position).is_some()),
            Err(error) if error.is_unsupported() => {}
            Err(error) => eprintln!("could not query the global cursor position: {error}"),
        }
        if let Some(display) = displays.primary().or_else(|| displays.displays().first()) {
            let bounds = display.bounds();
            let position = bounds.position();
            let extent = bounds.extent();
            let center = PhysicalPosition::new(
                position.x + f64::from(extent.width) / 2.0,
                position.y + f64::from(extent.height) / 2.0,
            );
            debug_assert!(displays.display(display.id()).is_some());
            debug_assert!(displays.display_nearest_point(center).is_some());
            debug_assert!(
                displays
                    .display_matching(display.work_area().unwrap_or(bounds))
                    .is_some()
            );
        }
        debug_assert_eq!(context.window_ids().len(), self.windows.len());
        #[cfg(any(target_os = "macos", target_os = "windows"))]
        debug_assert_eq!(context.child_windows(self.windows[0].handle.id()).len(), 1);
        let menu = MenuModel::new([MenuGroup::new(
            MenuItemId::new("demo").unwrap(),
            "Demo",
            [
                MenuEntry::Action(
                    MenuAction::new(MenuItemId::new("demo.ready").unwrap(), "Mark ready")
                        .with_checked(false)
                        .with_accelerator(
                            MenuAccelerator::parse("CommandOrControl+Shift+KeyR").unwrap(),
                        ),
                ),
                MenuEntry::Role(MenuRoleItem::new(MenuRole::Copy)),
                MenuEntry::Separator,
                MenuEntry::Action(MenuAction::new(
                    MenuItemId::new("demo.new-window").unwrap(),
                    "New window",
                )),
                MenuEntry::Action(MenuAction::new(
                    MenuItemId::new("demo.open-file").unwrap(),
                    "Open file…",
                )),
                MenuEntry::Action(MenuAction::new(
                    MenuItemId::new("demo.confirm").unwrap(),
                    "Show confirmation…",
                )),
                MenuEntry::Action(MenuAction::new(
                    MenuItemId::new("demo.close-window").unwrap(),
                    "Close last window",
                )),
                MenuEntry::Action(MenuAction::new(
                    MenuItemId::new("demo.relaunch").unwrap(),
                    "Relaunch and quit",
                )),
                MenuEntry::Action(MenuAction::new(
                    MenuItemId::new("demo.quit").unwrap(),
                    "Quit",
                )),
            ],
        )]);
        let _ = context.services().menus().set_application_menu(menu);
        let tray_menu = MenuModel::new([MenuGroup::new(
            MenuItemId::new("demo.tray").unwrap(),
            "Demo",
            [MenuEntry::Action(MenuAction::new(
                MenuItemId::new("demo.ready").unwrap(),
                "Mark ready",
            ))],
        )]);
        let tray_artwork = TrayIconImage::from_rgba([0x2f, 0x81, 0xf7, 0xff].repeat(256), 16, 16)
            .expect("valid demo tray artwork");
        let _ = context.services().tray().create(
            TrayOptions::new(TrayId::new("zui.demo").unwrap(), tray_artwork)
                .with_tooltip("ZUI native demo")
                .with_menu(tray_menu)
                .as_template(),
        );
        let _ = context
            .services()
            .global_shortcuts()
            .register(GlobalShortcut::new(
                GlobalShortcutId::new("demo.ready").unwrap(),
                ShortcutAccelerator::parse("CommandOrControl+Shift+KeyD").unwrap(),
            ));
        if let Some(window) = self.windows.first().map(|window| window.handle.id()) {
            match context
                .schedule_after(Duration::from_millis(250), DemoEvent::RenameWindow(window))
            {
                Ok(timer) => timer.detach(),
                Err(error) => eprintln!("could not schedule demo timer: {error}"),
            }
        }
        context.spawn(async { DemoEvent::BackgroundReady }).detach();
    }

    fn resumed(&mut self, _context: &mut AppContext<'_, DemoEvent>) {
        for window in &self.windows {
            let _ = window.handle.request_redraw();
        }
    }

    fn activated(&mut self, context: &mut AppContext<'_, DemoEvent>, event: ApplicationActivation) {
        if !event.has_visible_windows() {
            for window in &self.windows {
                let _ = window.handle.set_visible(true);
            }
            let _ = context.focus_application(ApplicationFocusOptions::new());
        }
        self.mark_ready();
    }

    fn display_event(&mut self, context: &mut AppContext<'_, DemoEvent>, event: DisplayEvent) {
        self.display_events += 1;
        let snapshot = context.display_snapshot();
        match &event {
            DisplayEvent::Added(display) | DisplayEvent::MetricsChanged { display, .. } => {
                debug_assert!(snapshot.display(display.id()).is_some());
            }
            DisplayEvent::Removed(display) => {
                debug_assert!(snapshot.display(display.id()).is_none());
            }
            _ => {}
        }
        for window in &self.windows {
            let _ = window.handle.request_redraw();
        }
    }

    fn window_event(&mut self, context: &mut WindowContext<'_, DemoEvent>, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => {
                context.close();
            }
            WindowEvent::FileDropped(path) => {
                let title = format!("zui app demo · dropped {}", path.display());
                if let Err(error) = context.window_handle().set_title(&title) {
                    context.exit_with_error(ApplicationError::product(
                        "dropped-file title update",
                        error,
                    ));
                }
            }
            _ => {}
        }
    }

    fn redraw(&mut self, context: &mut WindowContext<'_, DemoEvent>) {
        if self.background_ready.read(|ready| *ready)
            && let Err(error) = context
                .window_handle()
                .set_title("zui app demo · background ready")
        {
            context.exit_with_error(ApplicationError::product(
                "background-ready title update",
                error,
            ));
            return;
        }
        let Some(window) = self
            .windows
            .iter_mut()
            .find(|window| window.handle.id() == context.id())
        else {
            context.exit_with_error(ApplicationError::product(
                "demo component runtime lookup",
                std::io::Error::other("redraw target is not tracked"),
            ));
            return;
        };
        let frame =
            zui_demo::build_demo_frame_with_state(&self.background_ready, &mut window.components);
        if let Err(error) = context.present_frame(&frame, &zui::ui::UiDispatch::default()) {
            context.exit_with_error(ApplicationError::product("demo frame rendering", error));
        }
    }

    fn menu_action(&mut self, context: &mut AppContext<'_, DemoEvent>, action: MenuItemId) {
        match action.as_str() {
            "demo.ready" => {
                self.mark_ready();
            }
            "demo.new-window" => {
                let opened = self.application.proxy().open_window(
                    WindowOptions::new("zui app demo · asynchronous window")
                        .with_inner_size(LogicalSize::new(480.0, 240.0))
                        .with_resize_increments(LogicalSize::new(8.0, 8.0))
                        .with_position(LogicalPosition::new(160.0, 160.0)),
                );
                context
                    .spawn(async move { DemoEvent::WindowOpened(opened.await) })
                    .detach();
            }
            "demo.open-file" => {
                let Some(parent) = self.windows.first().map(|window| window.handle.clone()) else {
                    return;
                };
                let selected = context
                    .services()
                    .file_dialogs()
                    .open_file(FileDialogOptions::new().with_parent(parent));
                context
                    .spawn(async move { DemoEvent::FileSelected(selected.await) })
                    .detach();
            }
            "demo.confirm" => {
                let Some(parent) = self.windows.first().map(|window| window.handle.clone()) else {
                    return;
                };
                let answered = context.services().message_dialogs().show(
                    MessageDialogRequest::new("ZUI native dialog", "Mark the demo ready?")
                        .with_buttons(MessageDialogButtons::YesNo)
                        .with_parent(parent),
                );
                context
                    .spawn(async move { DemoEvent::MessageAnswered(answered.await) })
                    .detach();
            }
            "demo.close-window" => {
                if let Some(window) = self.windows.last()
                    && let Err(error) = window.handle.close()
                {
                    context.exit_with_error(ApplicationError::product(
                        "retained window close request",
                        error,
                    ));
                }
            }
            "demo.relaunch" => {
                if let Err(error) = context.relaunch() {
                    context.exit_with_error(ApplicationError::product(
                        "application relaunch scheduling",
                        error,
                    ));
                    return;
                }
                context.exit();
            }
            "demo.quit" => {
                if let Err(error) = self.application.proxy().exit() {
                    context.exit_with_error(ApplicationError::product(
                        "retained application exit request",
                        error,
                    ));
                }
            }
            _ => {}
        }
    }

    fn tray_event(&mut self, _context: &mut AppContext<'_, DemoEvent>, event: TrayEvent) {
        if matches!(event.kind, TrayEventKind::Click { .. }) {
            self.mark_ready();
        }
    }

    fn global_shortcut(
        &mut self,
        _context: &mut AppContext<'_, DemoEvent>,
        _event: GlobalShortcutEvent,
    ) {
        self.mark_ready();
    }

    fn open_url(&mut self, _context: &mut AppContext<'_, DemoEvent>, _url: ProtocolUrl) {
        self.mark_ready();
    }

    fn open_file(&mut self, _context: &mut AppContext<'_, DemoEvent>, _path: PathBuf) {
        self.mark_ready();
    }

    fn second_instance(&mut self, context: &mut AppContext<'_, DemoEvent>, event: SecondInstance) {
        debug_assert!(!event.arguments().is_empty());
        debug_assert_eq!(event.additional_data(), b"zui-demo");
        for window in &self.windows {
            let _ = window.handle.set_visible(true);
        }
        let _ = context.focus_application(ApplicationFocusOptions::new());
        self.mark_ready();
    }

    fn user_event(&mut self, context: &mut AppContext<'_, DemoEvent>, event: DemoEvent) {
        match event {
            DemoEvent::BackgroundReady => {
                self.mark_ready();
            }
            DemoEvent::FrameworkReady(Ok(())) => {
                debug_assert!(self.application.is_ready());
            }
            DemoEvent::FrameworkReady(Err(error)) => {
                context.exit_with_error(ApplicationError::product(
                    "application readiness wait",
                    error,
                ));
            }
            DemoEvent::FileIconLoaded(Ok(image)) => {
                debug_assert_eq!((image.width(), image.height()), (32, 32));
            }
            DemoEvent::FileIconLoaded(Err(error)) => {
                context.exit_with_error(ApplicationError::product(
                    "injected file icon lookup",
                    error,
                ));
            }
            DemoEvent::FileSelected(Ok(Some(path))) => {
                if let Some(window) = self.windows.first() {
                    let _ = window
                        .handle
                        .set_title(&format!("zui app demo · selected {}", path.display()));
                }
            }
            DemoEvent::FileSelected(Ok(None)) => {}
            DemoEvent::FileSelected(Err(error)) => {
                context.exit_with_error(ApplicationError::product(
                    "parented demo file dialog",
                    error,
                ));
            }
            DemoEvent::MessageAnswered(Ok(MessageDialogResponse::Yes)) => {
                self.mark_ready();
            }
            DemoEvent::MessageAnswered(Ok(_)) => {}
            DemoEvent::MessageAnswered(Err(error)) => {
                context.exit_with_error(ApplicationError::product(
                    "parented demo message dialog",
                    error,
                ));
            }
            DemoEvent::RenameWindow(target) => {
                if let Some(window) = self
                    .windows
                    .iter()
                    .find(|window| window.handle.id() == target)
                {
                    let _ = window.handle.set_title("zui app demo · timer fired");
                }
            }
            DemoEvent::WindowOpened(Ok(opened)) => {
                let window = opened.handle();
                if let Err(error) =
                    window.set_min_inner_logical_size(Some(LogicalSize::new(320.0, 180.0)))
                {
                    context.exit_with_error(ApplicationError::product(
                        "asynchronous demo window constraint",
                        error,
                    ));
                    return;
                }
                if let Err(error) = window.request_redraw() {
                    context.exit_with_error(ApplicationError::product(
                        "asynchronous demo window redraw",
                        error,
                    ));
                    return;
                }
                self.track_window(window);
            }
            DemoEvent::WindowOpened(Err(error)) => {
                context.exit_with_error(ApplicationError::product(
                    "asynchronous demo window creation",
                    error,
                ));
            }
        }
    }

    fn window_closed(&mut self, _context: &mut AppContext<'_, DemoEvent>, window: WindowId) {
        self.windows.retain(|entry| entry.handle.id() != window);
    }

    fn about_to_wait(&mut self, context: &mut AppContext<'_, DemoEvent>) {
        let _ = context.diagnostics().snapshot();
    }

    fn before_exit(
        &mut self,
        _context: &mut AppContext<'_, DemoEvent>,
        reason: ApplicationExitReason,
    ) -> ApplicationExitDecision {
        debug_assert!(reason.is_cancelable());
        ApplicationExitDecision::Exit
    }
}

fn main() -> ExitCode {
    let options = SingleInstanceOptions::new(
        SingleInstanceKey::new("dev.zeta.zui-demo").expect("valid demo single-instance key"),
    )
    .with_additional_data(b"zui-demo".to_vec());
    let outcome = match Application::builder()
        .with_application_name("ZUI Native Demo")
        .with_application_version("1.0.0-demo.1")
        .with_application_locale(ApplicationLocale::new("en-US").unwrap())
        .with_application_path_override(ApplicationPath::SessionData, std::env::temp_dir())
        .with_desktop_file_name(DesktopFileName::new("dev.zeta.zui-demo").unwrap())
        .with_application_badge(DemoSystemIntegration)
        .with_file_icons(DemoSystemIntegration)
        .with_jump_lists(DemoSystemIntegration)
        .with_login_items(DemoSystemIntegration)
        .with_protocol_clients(DemoSystemIntegration)
        .with_protocol_scheme(ProtocolScheme::new("zui-demo").unwrap())
        .with_diagnostics_inspection()
        .run_single_instance::<DemoEvent, _, _>(options, DemoApp::new)
    {
        Ok(outcome) => outcome,
        Err(error) => {
            eprintln!("zui native demo event loop failed: {error}");
            return ExitCode::FAILURE;
        }
    };
    let SingleInstanceRun::Primary(exit) = outcome else {
        return ExitCode::SUCCESS;
    };
    if let Some(error) = exit.error() {
        eprintln!("zui native demo failed: {error}");
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}
