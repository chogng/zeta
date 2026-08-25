use std::process::ExitCode;
use std::time::Duration;

use zui::app::App;
use zui::app::AppContext;
use zui::app::Application;
use zui::app::ApplicationError;
use zui::app::ProtocolScheme;
use zui::app::ProtocolUrl;
use zui::app::WindowContext;
use zui::services::GlobalShortcut;
use zui::services::GlobalShortcutEvent;
use zui::services::GlobalShortcutId;
use zui::services::MenuAction;
use zui::services::MenuEntry;
use zui::services::MenuGroup;
use zui::services::MenuItemId;
use zui::services::MenuModel;
use zui::services::ShortcutAccelerator;
use zui::services::TrayEvent;
use zui::services::TrayEventKind;
use zui::services::TrayIconImage;
use zui::services::TrayId;
use zui::services::TrayOptions;
use zui::window::LogicalSize;
use zui::window::WindowChrome;
use zui::window::WindowEvent;
use zui::window::WindowHandle;
use zui::window::WindowId;
use zui::window::WindowOptions;

enum DemoEvent {
    BackgroundReady,
    RenameWindow(WindowId),
}

#[derive(Default)]
struct DemoApp {
    windows: Vec<WindowHandle>,
    background_ready: bool,
}

impl App<DemoEvent> for DemoApp {
    fn resumed(&mut self, context: &mut AppContext<'_, DemoEvent>) {
        if !self.windows.is_empty() {
            return;
        }
        for title in ["zui app demo", "zui app demo · second window"] {
            let options = WindowOptions::new(title)
                .with_inner_size(LogicalSize::new(480.0, 240.0))
                .with_chrome(WindowChrome::Native);
            match context.open_window(options) {
                Ok(opened) => {
                    let window = opened.handle();
                    window.request_redraw();
                    self.windows.push(window);
                }
                Err(error) => {
                    context.exit_with_error(error);
                    return;
                }
            }
        }
        let menu = MenuModel::new([MenuGroup::new(
            MenuItemId::new("demo").unwrap(),
            "Demo",
            [MenuEntry::Action(MenuAction::new(
                MenuItemId::new("demo.ready").unwrap(),
                "Mark ready",
            ))],
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
        if let Some(window) = self.windows.first().and_then(WindowHandle::id) {
            match context
                .schedule_after(Duration::from_millis(250), DemoEvent::RenameWindow(window))
            {
                Ok(timer) => timer.detach(),
                Err(error) => eprintln!("could not schedule demo timer: {error}"),
            }
        }
        match context.spawn(async { DemoEvent::BackgroundReady }) {
            Ok(task) => task.detach(),
            Err(error) => eprintln!("could not start demo task: {error}"),
        }
    }

    fn window_event(&mut self, context: &mut WindowContext<'_, DemoEvent>, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => context.close(),
            WindowEvent::FileDropped(path) => {
                let title = format!("zui app demo · dropped {}", path.display());
                context.window_handle().set_title(&title);
            }
            _ => {}
        }
    }

    fn redraw(&mut self, context: &mut WindowContext<'_, DemoEvent>) {
        if self.background_ready {
            context
                .window_handle()
                .set_title("zui app demo · background ready");
        }
        let frame = zui_demo::build_demo_frame();
        let accessibility = frame
            .interaction()
            .accessibility_nodes(&zui::ui::UiDispatch::default());
        if let Err(error) = context.present_scene(frame.scene(), &accessibility) {
            context.exit_with_error(ApplicationError::product("demo frame rendering", error));
        }
    }

    fn menu_action(&mut self, _context: &mut AppContext<'_, DemoEvent>, action: MenuItemId) {
        if action.as_str() == "demo.ready" {
            self.background_ready = true;
            for window in &self.windows {
                window.request_redraw();
            }
        }
    }

    fn tray_event(&mut self, _context: &mut AppContext<'_, DemoEvent>, event: TrayEvent) {
        if matches!(event.kind, TrayEventKind::Click { .. }) {
            self.background_ready = true;
            for window in &self.windows {
                window.request_redraw();
            }
        }
    }

    fn global_shortcut(
        &mut self,
        _context: &mut AppContext<'_, DemoEvent>,
        _event: GlobalShortcutEvent,
    ) {
        self.background_ready = true;
        for window in &self.windows {
            window.request_redraw();
        }
    }

    fn open_url(&mut self, _context: &mut AppContext<'_, DemoEvent>, _url: ProtocolUrl) {
        self.background_ready = true;
        for window in &self.windows {
            window.request_redraw();
        }
    }

    fn user_event(&mut self, _context: &mut AppContext<'_, DemoEvent>, event: DemoEvent) {
        match event {
            DemoEvent::BackgroundReady => {
                self.background_ready = true;
            }
            DemoEvent::RenameWindow(target) => {
                if let Some(window) = self
                    .windows
                    .iter()
                    .find(|window| window.id() == Some(target))
                {
                    window.set_title("zui app demo · timer fired");
                }
            }
        }
        for window in &self.windows {
            window.request_redraw();
        }
    }

    fn about_to_wait(&mut self, context: &mut AppContext<'_, DemoEvent>) {
        let _ = context.diagnostics().snapshot();
    }
}

fn main() -> ExitCode {
    let exit = match Application::builder()
        .with_protocol_scheme(ProtocolScheme::new("zui-demo").unwrap())
        .with_diagnostics_inspection()
        .run::<DemoEvent, _, _>(|_| DemoApp::default())
    {
        Ok(exit) => exit,
        Err(error) => {
            eprintln!("zui native demo event loop failed: {error}");
            return ExitCode::FAILURE;
        }
    };
    if let Some(error) = exit.error() {
        eprintln!("zui native demo failed: {error}");
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}
