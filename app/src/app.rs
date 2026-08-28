//! Desktop product composition and lifecycle.
//!
//! This module coordinates zui lifecycle callbacks, feature hosts, Workbench state, and the final
//! frame. Domain modules remain responsible for their own state and adapters.

use std::process::ExitCode;
use std::time::Instant;

use crate::app_server::{AppServerHost, AppServerRequestHandle, local_profile_root};
use crate::file_editor_input::FileEditorInputState;
use crate::git_branch_context_menu::GitBranchContextMenuState;
use crate::keybindings::{KeybindingsResource, KeybindingsResourcePoll};
use crate::launch::AppLaunch;
use crate::product_event::ProductEvent;
use crate::remote_connection_cli::AppInvocation;
use crate::remote_tunnel_process::ProductRemoteTunnelHost;
use crate::session_host::SessionRuntime;
use crate::shell_interaction::{COMPOSER, FILE_EDITOR_DOCUMENT};
use crate::shell_scene::{
    ShellPresentation, ShellPresentationModel, build_shell_presentation_with_animation_bindings,
    rebuild_shell_overlays, terminal_grid_size_for_bounds, terminal_grid_size_for_viewport,
    terminal_pane_bounds_for_viewport, terminal_pane_sash_for_viewport,
};
use crate::terminal_session::{TerminalSession, TerminalSessionEvent, TerminalSessionKey};
use crate::workspace_context::WorkspaceContext;
use crate::workspace_path_picker::WorkspacePathPickerState;
use crate::workspace_surface::WorkspaceSurface;
use zeta_editor::CodeEditorStyle;
use zeta_editor_host::FileEditorHost;
use zeta_editor_host::FileEditorSearchState;
use zeta_files::{FilesAction, FilesState};
use zeta_protocol::SessionId;
use zeta_scm::{ScmDiff, ScmState};
use zeta_session::SessionPaneState;
use zeta_settings::RemoteConnectionManagerState;
use zeta_settings::RemoteConnectionPickerState;
use zeta_settings::RemoteTunnelManagerState;
use zeta_settings::SettingsState;
use zeta_terminal::{BlockStatus, GridSize, ScreenBuffer};
use zeta_terminal_workspace::PaneBinding;
use zeta_terminal_workspace::TerminalPaneViewState;
use zeta_terminal_workspace::TerminalPaneViews;
use zeta_theme::{ColorScheme, ThemeLoadOptions, ThemeLoader, ThemeSurface, default_device_root};
use zeta_ui_components::{SashOrientation, SashPointerPresence};
use zeta_ui_theme::{DEFAULT_UI_THEME, UiTheme};
use zeta_workbench::SessionSearchState;
use zeta_workbench::{
    LogicalViewport, PaneGroupId as PaneId, PaneInput, PaneInputKind, PaneKey, PaneSplitDirection,
    PaneSplitId, TabInputKey, WorkbenchHost,
};
use zui::ui::{CaretBlinkAdvance, CaretBlinkController, Point, TextInputLayoutEngine};
use zui::ui::{SplitViewOrientation, SplitViewResizeSnapshot};

type TerminalWorkspace =
    zeta_terminal_workspace::TerminalWorkspace<TerminalSession, TerminalSessionEvent>;
type TerminalReadyOutcome = zeta_terminal_workspace::TerminalReadyOutcome<TerminalSessionEvent>;
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
#[path = "app/command_dispatch.rs"]
pub(crate) mod command_dispatch;
#[path = "app/events.rs"]
mod events;
#[path = "features/editor/file_editor_input.rs"]
pub(crate) mod file_editor_input;
#[path = "features/editor/file_editor_pane.rs"]
pub(crate) mod file_editor_pane;
#[path = "app/frame.rs"]
mod frame;
#[path = "features/workspace/git_branch_context_menu.rs"]
pub(crate) mod git_branch_context_menu;
#[path = "features/workspace/git_branch_context_menu_input.rs"]
pub(crate) mod git_branch_context_menu_input;
#[path = "platform/input_method.rs"]
pub(crate) mod input_method;
#[path = "app/interaction.rs"]
mod interaction;
#[path = "platform/keybindings.rs"]
pub(crate) mod keybindings;
#[path = "features/editor/language_service_host.rs"]
pub(crate) mod language_service_host;
#[path = "features/remote/launch.rs"]
pub(crate) mod launch;
#[cfg(test)]
#[path = "features/remote/launch_profile_tests.rs"]
pub(crate) mod launch_profile_tests;
#[path = "features/remote/launch_progress.rs"]
pub(crate) mod launch_progress;
#[cfg(test)]
#[path = "features/remote/launch_progress_tests.rs"]
pub(crate) mod launch_progress_tests;
#[cfg(test)]
#[path = "features/remote/launch_test_support.rs"]
pub(crate) mod launch_test_support;
#[cfg(test)]
#[path = "features/remote/launch_tests.rs"]
pub(crate) mod launch_tests;
#[path = "app/lifecycle.rs"]
mod lifecycle;
#[path = "app/mouse_wheel.rs"]
pub(crate) mod mouse_wheel;
#[path = "app/presentation.rs"]
mod presentation;
#[path = "platform/product_event.rs"]
pub(crate) mod product_event;
#[path = "features/remote/remote_connection_cli.rs"]
pub(crate) mod remote_connection_cli;
#[cfg(test)]
#[path = "features/remote/remote_connection_cli_tests.rs"]
pub(crate) mod remote_connection_cli_tests;
#[path = "features/remote/remote_connection_launch_input.rs"]
pub(crate) mod remote_connection_launch_input;
#[path = "features/remote/remote_connection_manager_input.rs"]
pub(crate) mod remote_connection_manager_input;
#[path = "features/remote/remote_connection_picker_input.rs"]
pub(crate) mod remote_connection_picker_input;
#[path = "features/remote/remote_connection_process.rs"]
pub(crate) mod remote_connection_process;
#[path = "features/remote/remote_connection_tunnel.rs"]
pub(crate) mod remote_connection_tunnel;
#[cfg(test)]
#[path = "features/remote/remote_connection_tunnel_tests.rs"]
pub(crate) mod remote_connection_tunnel_tests;
#[path = "features/remote/remote_tunnel_manager_input.rs"]
pub(crate) mod remote_tunnel_manager_input;
#[path = "features/remote/remote_tunnel_process.rs"]
pub(crate) mod remote_tunnel_process;
#[path = "app/run.rs"]
mod run;
#[path = "app/runtime.rs"]
mod runtime;
#[path = "features/agent/session_catalog.rs"]
pub(crate) mod session_catalog;
#[path = "features/agent/session_host.rs"]
pub(crate) mod session_host;
#[path = "presentation/shell_interaction.rs"]
pub(crate) mod shell_interaction;
#[path = "presentation/shell_scene.rs"]
pub(crate) mod shell_scene;
#[path = "app/state.rs"]
mod state;
#[path = "presentation/tab_context_menu.rs"]
pub(crate) mod tab_context_menu;
#[path = "features/terminal/terminal_blocks.rs"]
pub(crate) mod terminal_blocks;
#[path = "features/terminal/terminal_history.rs"]
pub(crate) mod terminal_history;
#[path = "features/terminal/terminal_input.rs"]
pub(crate) mod terminal_input;
#[path = "features/terminal/terminal_output_scroll_view.rs"]
pub(crate) mod terminal_output_scroll_view;
#[path = "features/terminal/terminal_pointer.rs"]
pub(crate) mod terminal_pointer;
#[path = "features/terminal/terminal_selection.rs"]
pub(crate) mod terminal_selection;
#[path = "features/terminal/terminal_session.rs"]
pub(crate) mod terminal_session;
#[path = "features/agent/thread_timeline_scroll.rs"]
pub(crate) mod thread_timeline_scroll;
mod workbench;
mod workbench_resize;
mod workbench_tabs_resize;
#[path = "features/workspace/workspace_context.rs"]
pub(crate) mod workspace_context;
#[path = "features/workspace/workspace_path_picker.rs"]
pub(crate) mod workspace_path_picker;
#[path = "features/workspace/workspace_path_picker_input.rs"]
pub(crate) mod workspace_path_picker_input;
#[path = "features/workspace/workspace_surface.rs"]
pub(crate) mod workspace_surface;

pub use run::run;
pub(crate) use state::ProductApp;

pub(crate) const PRODUCT_DISPLAY_NAME: &str = "app";
const DEFAULT_THEME_ENTRY: &str = "app";
const INITIAL_WIDTH: f64 = 1_280.0;
const INITIAL_HEIGHT: f64 = 800.0;
