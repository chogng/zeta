//! Workbench application composition and lifecycle.
//!
//! This module coordinates zui lifecycle callbacks, feature hosts, Workbench state, and the final
//! frame. Domain modules remain responsible for their own state and adapters.

use std::process::ExitCode;
use std::sync::Arc;
use std::time::Instant;

use crate::MainSurface;
use crate::PaneBinding;
use crate::SessionSearchState;
use crate::app_server::{AppServerHost, AppServerRequestHandle};
use crate::directory_picker::DirectoryPickerState;
use crate::environment_context::EnvironmentContext;
use crate::git_branch_picker::GitBranchPickerState;
use crate::launch::AppLaunch;
use crate::remote_connection_cli::AppInvocation;
use crate::remote_tunnel_process::RemoteTunnelHost;
use crate::session_host::SessionRuntime;
use crate::workbench_event::WorkbenchEvent;
use crate::{
    LogicalViewport, PaneGroupId as PaneId, PaneInput, PaneInputKind, PaneKey, PaneSplitDirection,
    PaneSplitId, TabInputKey, WorkbenchHost,
};
use crate::{
    WorkbenchPresentation, WorkbenchPresentationModel,
    build_workbench_presentation_with_animation_bindings, rebuild_workbench_overlays,
    terminal_grid_size_for_bounds, terminal_grid_size_for_viewport,
    terminal_pane_bounds_for_viewport, terminal_pane_sash_for_viewport,
};
use zeta_app_server_protocol::protocol::config::FrontendConfigDto;
use zeta_editor::CodeEditorStyle;
use zeta_editor_host::FILE_EDITOR_DOCUMENT;
use zeta_editor_host::FileEditorHost;
use zeta_editor_host::FileEditorInputState;
use zeta_editor_host::FileEditorLanguageService;
use zeta_editor_host::FileEditorSearchState;
use zeta_files::{FilesAction, FilesState};
use zeta_protocol::SessionId;
use zeta_scm::{ScmDiff, ScmState};
use zeta_session::SessionPaneState;
use zeta_session::interaction::COMPOSER;
use zeta_settings::RemoteConnectionManagerState;
use zeta_settings::RemoteConnectionPickerState;
use zeta_settings::RemoteTunnelManagerState;
use zeta_settings::SettingsState;
use zeta_terminal::{BlockStatus, GridSize, ScreenBuffer};
use zeta_terminal_runtime::TerminalPaneViewState;
use zeta_terminal_runtime::TerminalPaneViews;
use zeta_terminal_runtime::{TerminalSession, TerminalSessionEvent, TerminalSessionKey};
use zeta_theme::{ColorScheme, ThemeLoadOptions, ThemeLoader, default_device_root};
use zeta_ui_components::SashOrientation;
use zeta_ui_theme::{DEFAULT_UI_THEME, UiTheme};
use zui::ui::{
    CaretBlinkAdvance, CaretBlinkController, Color, FontFamily, Point, TextInputLayoutEngine,
    TextStyle,
};
use zui::ui::{SplitViewOrientation, SplitViewResizeSnapshot};

type TerminalRuntime =
    zeta_terminal_runtime::TerminalRuntime<TerminalSession, TerminalSessionEvent>;
type TerminalReadyOutcome = zeta_terminal_runtime::TerminalReadyOutcome<TerminalSessionEvent>;
use zui::app::AccessibilityAction;
use zui::app::AccessibilityActionKind;
use zui::app::App;
use zui::app::AppContext;
use zui::app::Application;
use zui::app::ApplicationError;
use zui::app::ApplicationHandle;
use zui::app::ControlFlow;
use zui::app::WindowContext;
use zui::input::ElementState;
use zui::input::ModifiersState;
use zui::input::MouseButton;
use zui::services::ClipboardHandle;
use zui::ui::CursorFeedback;
use zui::ui::DispatchInvalidation;
use zui::ui::DispatchOutcome;
use zui::ui::ElementId;
use zui::ui::FrameDeadlineSet;
use zui::ui::FrameInvalidation;
use zui::ui::FrameSchedule;
use zui::ui::FrameScheduler;
use zui::ui::HoverPresence;
use zui::ui::RetainedRuntime;
use zui::ui::UiDispatch;
use zui::ui::UiIntent;
use zui::window::CursorIcon;
use zui::window::LogicalSize;
use zui::window::PhysicalExtent;
use zui::window::Theme;
use zui::window::WindowChrome;
use zui::window::WindowControlInsets;
use zui::window::WindowEvent;
use zui::window::WindowHandle;
use zui::window::WindowOptions;

#[path = "app_server.rs"]
pub(crate) mod app_server;
#[path = "command.rs"]
pub(crate) mod command;
#[path = "application/events.rs"]
mod events;
#[path = "editor/file_editor_input.rs"]
pub(crate) mod file_editor_input;
pub(crate) use zeta_editor_host as file_editor_pane;
#[path = "application/frame.rs"]
mod frame;
pub(crate) use zeta_scm as git_branch_picker;
#[path = "environment/git_branch_picker_input.rs"]
pub(crate) mod git_branch_picker_input;
#[path = "platform/input_method.rs"]
pub(crate) mod input_method;
#[path = "application/interaction.rs"]
mod interaction;
#[path = "platform/keybindings.rs"]
pub(crate) mod keybindings;
#[path = "editor/language_service_adapter.rs"]
pub(crate) mod language_service_adapter;
#[path = "remote/launch.rs"]
pub(crate) mod launch;
#[cfg(test)]
#[path = "remote/launch_profile_tests.rs"]
pub(crate) mod launch_profile_tests;
#[path = "remote/launch_progress.rs"]
pub(crate) mod launch_progress;
#[cfg(test)]
#[path = "remote/launch_progress_tests.rs"]
pub(crate) mod launch_progress_tests;
#[cfg(test)]
#[path = "remote/launch_test_support.rs"]
pub(crate) mod launch_test_support;
#[cfg(test)]
#[path = "remote/launch_tests.rs"]
pub(crate) mod launch_tests;
#[path = "application/lifecycle.rs"]
mod lifecycle;
#[path = "application/mouse_wheel.rs"]
pub(crate) mod mouse_wheel;
#[path = "application/presentation.rs"]
mod presentation;
#[path = "remote/remote_connection_cli.rs"]
pub(crate) mod remote_connection_cli;
#[cfg(test)]
#[path = "remote/remote_connection_cli_tests.rs"]
pub(crate) mod remote_connection_cli_tests;
#[path = "remote/remote_connection_launch_input.rs"]
pub(crate) mod remote_connection_launch_input;
#[path = "remote/remote_connection_manager_input.rs"]
pub(crate) mod remote_connection_manager_input;
#[path = "remote/remote_connection_picker_input.rs"]
pub(crate) mod remote_connection_picker_input;
#[path = "remote/remote_connection_process.rs"]
pub(crate) mod remote_connection_process;
#[path = "remote/remote_connection_tunnel.rs"]
pub(crate) mod remote_connection_tunnel;
#[cfg(test)]
#[path = "remote/remote_connection_tunnel_tests.rs"]
pub(crate) mod remote_connection_tunnel_tests;
#[path = "remote/remote_tunnel_manager_input.rs"]
pub(crate) mod remote_tunnel_manager_input;
#[path = "remote/remote_tunnel_process.rs"]
pub(crate) mod remote_tunnel_process;
#[path = "application/run.rs"]
mod run;
#[path = "application/runtime.rs"]
mod runtime;
use self::runtime::GuiConfig;
#[cfg(test)]
#[path = "application/runtime_tests.rs"]
mod runtime_tests;
#[path = "application/scm_input.rs"]
mod scm_input;
#[path = "agent/session_host.rs"]
pub(crate) mod session_host;
#[path = "application/state.rs"]
mod state;
#[path = "application/tab_context_menu.rs"]
pub(crate) mod tab_context_menu;
#[path = "platform/workbench_event.rs"]
pub(crate) mod workbench_event;
pub(crate) use zeta_terminal_runtime as terminal_blocks;
pub(crate) use zeta_terminal_runtime as terminal_history;
#[path = "terminal/terminal_input.rs"]
pub(crate) mod terminal_input;
pub(crate) use zeta_terminal_runtime as terminal_output_scroll_view;
#[path = "terminal/terminal_pointer.rs"]
pub(crate) mod terminal_pointer;
#[path = "terminal/terminal_selection.rs"]
pub(crate) mod terminal_selection;
pub(crate) use zeta_terminal_runtime as terminal_session;
#[path = "environment/directory_picker.rs"]
pub(crate) mod directory_picker;
#[path = "environment/directory_picker_input.rs"]
pub(crate) mod directory_picker_input;
#[path = "environment/environment_context.rs"]
pub(crate) mod environment_context;
#[path = "agent/thread_timeline_scroll.rs"]
pub(crate) mod thread_timeline_scroll;
mod workbench;
mod workbench_resize;
mod workbench_tabs_resize;
pub use run::run;
pub(crate) use state::WorkbenchApplication;

pub(crate) const APP_DISPLAY_NAME: &str = "app";
const DEFAULT_THEME_ENTRY: &str = "app";
const INITIAL_WIDTH: f64 = 1_280.0;
const INITIAL_HEIGHT: f64 = 800.0;
