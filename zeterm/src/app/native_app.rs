use std::collections::HashMap;
use std::process::ExitCode;
use std::time::Instant;

use crate::agent_session::AgentSession;
use crate::app_server::{AppServerHost, local_profile_root};
use crate::file_editor_host::FileEditorHost;
use crate::file_editor_input::FileEditorInputState;
use crate::git_branch_context_menu::GitBranchContextMenuState;
use crate::keybindings::{KeybindingsResource, KeybindingsResourcePoll};
use crate::keyboard_shortcuts::KeyboardShortcutsState;
use crate::language_server_settings::LanguageServerSettingsState;
use crate::launch::ZetermLaunch;
use crate::native_event::NativeEvent;
use crate::pane_group::{PaneGroup, PaneId, PaneSplitDirection, PaneSplitId};
use crate::pane_host::{PaneHost, PaneHostScope};
use crate::pane_input::{PaneBinding, PaneInput, PaneInputKind};
use crate::remote_connection_cli::ZetermInvocation;
use crate::remote_connection_manager::RemoteConnectionManagerState;
use crate::remote_connection_picker::RemoteConnectionPickerState;
use crate::remote_tunnel_manager::RemoteTunnelManagerState;
use crate::remote_tunnel_process::NativeRemoteTunnelHost;
use crate::session::session_context_menu::SessionContextMenuState;
use crate::session::session_search::SessionSearch;
use crate::session::session_sidebar::SessionSidebarState;
use crate::session::session_switch_trace;
use crate::shell_interaction::{COMPOSER, FILE_EDITOR_DOCUMENT};
use crate::shell_scene::{
    ShellPresentation, ShellPresentationModel, build_shell_presentation_with_animation_bindings,
    rebuild_shell_fragment, rebuild_shell_overlays, terminal_grid_size_for_bounds,
    terminal_grid_size_for_viewport, terminal_pane_bounds_for_viewport,
    terminal_pane_sash_for_viewport,
};
use crate::shell_style::{SHELL_PALETTE, ShellPalette, code_editor_style};
use crate::sidebar_pane_workspace::{AgentSidebarView, SidebarPaneWorkspace};
use crate::sidebar_part::SidebarPartState;
use crate::tab_input::{TabInputKey, TabInputModel};
use crate::terminal_pane_view::TerminalPaneViewState;
use crate::terminal_pointer::TerminalPointer;
use crate::terminal_scrollback::TerminalScroll;
use crate::terminal_selection::TerminalSelection;
use crate::terminal_session::{TerminalSession, TerminalSessionEvent, TerminalSessionKey};
use crate::thread_projection::ThreadProjection;
use crate::thread_timeline_scroll::ThreadTimelineScroll;
use crate::workbench::terminal_workspace::{TerminalReadyOutcome, TerminalWorkspace};
use crate::workspace_context::WorkspaceContext;
use crate::workspace_path_picker::WorkspacePathPickerState;
use crate::workspace_surface::WorkspaceSurface;
use zeta_agent_sidebar::AgentSidebarAction;
use zeta_composer::Composer;
use zeta_editor::CodeEditorStyle;
use zeta_protocol::SessionId;
use zeta_settings::SettingsPageSection;
use zeta_terminal::{BlockStatus, GridSize, ScreenBuffer};
use zeta_theme::{ColorScheme, ThemeLoadOptions, ThemeLoader, ThemeSurface, default_device_root};
use zeta_ui::layout::LogicalViewport;
use zeta_ui::{CaretBlinkAdvance, CaretBlinkController, Point, TextInputLayoutEngine};
use zeta_ui::{
    Resizable, SashOrientation, SashPointerPresence, SplitViewOrientation, SplitViewResizeSnapshot,
};
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

#[path = "../features/agent/agent_session.rs"]
pub(crate) mod agent_session;
#[path = "../app_server.rs"]
pub(crate) mod app_server;
#[path = "command_dispatch.rs"]
pub(crate) mod command_dispatch;
#[path = "../features/agent/composer_host.rs"]
pub(crate) mod composer_host;
#[path = "../features/agent/composer_panel.rs"]
pub(crate) mod composer_panel;
#[path = "events.rs"]
mod events;
#[path = "../features/editor/file_editor_auto_scroll.rs"]
pub(crate) mod file_editor_auto_scroll;
#[path = "../features/editor/file_editor_diagnostics.rs"]
pub(crate) mod file_editor_diagnostics;
#[path = "../features/editor/file_editor_host.rs"]
pub(crate) mod file_editor_host;
#[path = "../features/editor/file_editor_input.rs"]
pub(crate) mod file_editor_input;
#[path = "../features/editor/file_editor_language_features.rs"]
pub(crate) mod file_editor_language_features;
#[path = "../features/editor/file_editor_pane.rs"]
pub(crate) mod file_editor_pane;
#[path = "../features/editor/file_editor_search.rs"]
pub(crate) mod file_editor_search;
#[path = "frame.rs"]
mod frame;
#[path = "../features/workspace/git_branch_context_menu.rs"]
pub(crate) mod git_branch_context_menu;
#[path = "../features/workspace/git_branch_context_menu_input.rs"]
pub(crate) mod git_branch_context_menu_input;
#[path = "../features/workspace/input_context_toolbar.rs"]
pub(crate) mod input_context_toolbar;
#[path = "../platform/input_method.rs"]
pub(crate) mod input_method;
#[path = "interaction.rs"]
mod interaction;
#[path = "../platform/keybindings.rs"]
pub(crate) mod keybindings;
#[path = "../features/settings/keyboard_shortcuts.rs"]
pub(crate) mod keyboard_shortcuts;
#[path = "../features/settings/language_server_settings.rs"]
pub(crate) mod language_server_settings;
#[path = "../features/settings/language_server_settings_input.rs"]
pub(crate) mod language_server_settings_input;
#[path = "../features/editor/language_service_host.rs"]
pub(crate) mod language_service_host;
#[path = "../features/remote/launch.rs"]
pub(crate) mod launch;
#[cfg(test)]
#[path = "../features/remote/launch_profile_tests.rs"]
pub(crate) mod launch_profile_tests;
#[path = "../features/remote/launch_progress.rs"]
pub(crate) mod launch_progress;
#[cfg(test)]
#[path = "../features/remote/launch_progress_tests.rs"]
pub(crate) mod launch_progress_tests;
#[cfg(test)]
#[path = "../features/remote/launch_test_support.rs"]
pub(crate) mod launch_test_support;
#[cfg(test)]
#[path = "../features/remote/launch_tests.rs"]
pub(crate) mod launch_tests;
#[path = "lifecycle.rs"]
mod lifecycle;
#[path = "../platform/native_event.rs"]
pub(crate) mod native_event;
#[path = "../workbench/pane_group.rs"]
pub(crate) mod pane_group;
#[path = "../workbench/pane_host.rs"]
pub(crate) mod pane_host;
#[path = "../workbench/pane_input.rs"]
pub(crate) mod pane_input;
#[path = "presentation.rs"]
mod presentation;
#[path = "../features/remote/remote_connection_cli.rs"]
pub(crate) mod remote_connection_cli;
#[cfg(test)]
#[path = "../features/remote/remote_connection_cli_tests.rs"]
pub(crate) mod remote_connection_cli_tests;
#[path = "../features/remote/remote_connection_launch_input.rs"]
pub(crate) mod remote_connection_launch_input;
#[path = "../features/remote/remote_connection_manager.rs"]
pub(crate) mod remote_connection_manager;
#[path = "../features/remote/remote_connection_manager_input.rs"]
pub(crate) mod remote_connection_manager_input;
#[path = "../features/remote/remote_connection_manager_view.rs"]
pub(crate) mod remote_connection_manager_view;
#[path = "../features/remote/remote_connection_picker.rs"]
pub(crate) mod remote_connection_picker;
#[path = "../features/remote/remote_connection_picker_input.rs"]
pub(crate) mod remote_connection_picker_input;
#[path = "../features/remote/remote_connection_process.rs"]
pub(crate) mod remote_connection_process;
#[path = "../features/remote/remote_connection_tunnel.rs"]
pub(crate) mod remote_connection_tunnel;
#[cfg(test)]
#[path = "../features/remote/remote_connection_tunnel_tests.rs"]
pub(crate) mod remote_connection_tunnel_tests;
#[path = "../features/remote/remote_tunnel_manager.rs"]
pub(crate) mod remote_tunnel_manager;
#[path = "../features/remote/remote_tunnel_manager_input.rs"]
pub(crate) mod remote_tunnel_manager_input;
#[path = "../features/remote/remote_tunnel_manager_view.rs"]
pub(crate) mod remote_tunnel_manager_view;
#[path = "../features/remote/remote_tunnel_process.rs"]
pub(crate) mod remote_tunnel_process;
#[path = "run.rs"]
mod run;
#[path = "runtime.rs"]
mod runtime;
#[path = "../session.rs"]
pub(crate) mod session;
#[path = "../features/settings/settings_sections.rs"]
pub(crate) mod settings_sections;
#[path = "../presentation/shell_interaction.rs"]
pub(crate) mod shell_interaction;
#[path = "../presentation/shell_scene.rs"]
pub(crate) mod shell_scene;
#[path = "../presentation/shell_style.rs"]
pub(crate) mod shell_style;
#[path = "../workbench/sidebar_pane_workspace.rs"]
pub(crate) mod sidebar_pane_workspace;
#[path = "../workbench/sidebar_part.rs"]
pub(crate) mod sidebar_part;
#[path = "state.rs"]
mod state;
#[path = "../workbench/tab_input.rs"]
pub(crate) mod tab_input;
#[path = "../features/terminal/terminal_blocks.rs"]
pub(crate) mod terminal_blocks;
#[path = "../features/terminal/terminal_input.rs"]
pub(crate) mod terminal_input;
#[path = "../features/terminal/terminal_output_scroll_view.rs"]
pub(crate) mod terminal_output_scroll_view;
#[path = "../features/terminal/terminal_pane_view.rs"]
pub(crate) mod terminal_pane_view;
#[path = "../features/terminal/terminal_pointer.rs"]
pub(crate) mod terminal_pointer;
#[path = "../features/terminal/terminal_projection.rs"]
pub(crate) mod terminal_projection;
#[path = "../features/terminal/terminal_scrollback.rs"]
pub(crate) mod terminal_scrollback;
#[path = "../features/terminal/terminal_selection.rs"]
pub(crate) mod terminal_selection;
#[path = "../features/terminal/terminal_session.rs"]
pub(crate) mod terminal_session;
#[path = "../features/agent/thread_projection.rs"]
pub(crate) mod thread_projection;
#[path = "../features/agent/thread_timeline.rs"]
pub(crate) mod thread_timeline;
#[path = "../features/agent/thread_timeline_scroll.rs"]
pub(crate) mod thread_timeline_scroll;
#[path = "../workbench/titlebar.rs"]
pub(crate) mod titlebar;
#[path = "../workbench.rs"]
pub(crate) mod workbench;
#[path = "workbench.rs"]
mod workbench_runtime;
#[path = "../features/workspace/workspace_context.rs"]
pub(crate) mod workspace_context;
#[path = "../features/workspace/workspace_path_picker.rs"]
pub(crate) mod workspace_path_picker;
#[path = "../features/workspace/workspace_path_picker_input.rs"]
pub(crate) mod workspace_path_picker_input;
#[path = "../features/workspace/workspace_surface.rs"]
pub(crate) mod workspace_surface;

pub use run::run;
pub(crate) use state::NativeApp;
use state::TerminalPaneResize;

pub(crate) const PRODUCT_DISPLAY_NAME: &str = "zeterm";
const DEFAULT_THEME_ENTRY: &str = "zeterm";
const INITIAL_WIDTH: f64 = 1_280.0;
const INITIAL_HEIGHT: f64 = 800.0;
