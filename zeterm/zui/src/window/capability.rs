use std::error::Error;
use std::fmt;
use std::sync::Arc;
use std::sync::Weak;

use winit::dpi::LogicalPosition;
use winit::window::Window;

use crate::devtools::DevToolsHandle;

use super::CursorIcon;
use super::ImeCursorArea;
use super::LogicalSize;
use super::PhysicalExtent;
use super::Theme;
use super::WindowChrome;
use super::WindowControlInsets;
use super::WindowId;
use super::chrome::window_control_insets;

/// Failure while applying an operation through a non-owning window capability.
#[derive(Debug)]
pub enum WindowOperationError {
    /// The runtime released the native window before the operation was requested.
    Closed {
        window: WindowId,
        operation: &'static str,
    },
    /// A logical size supplied to the operation was invalid.
    InvalidSize {
        window: WindowId,
        operation: &'static str,
    },
    /// The platform rejected an otherwise valid operation.
    Platform {
        window: WindowId,
        operation: &'static str,
        source: Box<dyn Error + Send + Sync>,
    },
}

impl WindowOperationError {
    /// Returns the stable identity of the operation target.
    pub const fn window(&self) -> WindowId {
        match self {
            Self::Closed { window, .. }
            | Self::InvalidSize { window, .. }
            | Self::Platform { window, .. } => *window,
        }
    }

    /// Returns the stable name of the failed operation.
    pub const fn operation(&self) -> &'static str {
        match self {
            Self::Closed { operation, .. }
            | Self::InvalidSize { operation, .. }
            | Self::Platform { operation, .. } => operation,
        }
    }

    /// Returns whether the runtime no longer owns the target window.
    pub const fn is_closed(&self) -> bool {
        matches!(self, Self::Closed { .. })
    }
}

impl fmt::Display for WindowOperationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Closed { window, operation } => write!(
                formatter,
                "{operation} failed: window {} is closed",
                window.into_raw()
            ),
            Self::InvalidSize { operation, .. } => write!(
                formatter,
                "{operation} failed: logical size must be finite and positive"
            ),
            Self::Platform {
                operation, source, ..
            } => write!(formatter, "{operation} failed: {source}"),
        }
    }
}

impl Error for WindowOperationError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Platform { source, .. } => Some(source.as_ref()),
            Self::Closed { .. } | Self::InvalidSize { .. } => None,
        }
    }
}

/// Queryable platform state captured from one live native window.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WindowState {
    physical_extent: PhysicalExtent,
    logical_size: LogicalSize,
    scale_factor: f64,
    visible: Option<bool>,
    focused: bool,
    minimized: Option<bool>,
    maximized: bool,
    fullscreen: bool,
    resizable: bool,
    theme: Option<Theme>,
}

impl WindowState {
    fn from_native(window: &Window) -> Self {
        let extent = window.inner_size();
        let scale_factor = window.scale_factor();
        let scale_factor = if scale_factor.is_finite() && scale_factor > 0.0 {
            scale_factor
        } else {
            1.0
        };
        Self {
            physical_extent: PhysicalExtent::new(extent.width, extent.height),
            logical_size: LogicalSize::new(
                f64::from(extent.width) / scale_factor,
                f64::from(extent.height) / scale_factor,
            ),
            scale_factor,
            visible: window.is_visible(),
            focused: window.has_focus(),
            minimized: window.is_minimized(),
            maximized: window.is_maximized(),
            fullscreen: window.fullscreen().is_some(),
            resizable: window.is_resizable(),
            theme: window.theme().map(Theme::from_native),
        }
    }

    /// Returns the current physical content extent.
    pub const fn physical_extent(self) -> PhysicalExtent {
        self.physical_extent
    }

    /// Returns the current logical content size.
    pub const fn logical_size(self) -> LogicalSize {
        self.logical_size
    }

    /// Returns the validated logical-to-physical scale factor.
    pub const fn scale_factor(self) -> f64 {
        self.scale_factor
    }

    /// Returns platform visibility when the backend can report it.
    pub const fn visible(self) -> Option<bool> {
        self.visible
    }

    /// Returns whether the window currently owns keyboard focus.
    pub const fn focused(self) -> bool {
        self.focused
    }

    /// Returns platform minimization state when the backend can report it.
    pub const fn minimized(self) -> Option<bool> {
        self.minimized
    }

    /// Returns whether the platform currently reports the window as maximized.
    pub const fn maximized(self) -> bool {
        self.maximized
    }

    /// Returns whether the window currently occupies a fullscreen space.
    pub const fn fullscreen(self) -> bool {
        self.fullscreen
    }

    /// Returns whether the user can resize the window.
    pub const fn resizable(self) -> bool {
        self.resizable
    }

    /// Returns the current platform appearance preference when available.
    pub const fn theme(self) -> Option<Theme> {
        self.theme
    }
}

/// Non-owning platform capability for updating a live native window.
///
/// Application runtimes retain canonical window ownership. Product state may keep this handle to
/// request redraws or forward UI decisions without extending the native window lifecycle.
#[derive(Clone)]
pub struct WindowHandle {
    id: WindowId,
    window: Weak<Window>,
    chrome: WindowChrome,
    devtools: DevToolsHandle,
}

impl WindowHandle {
    pub(crate) fn new(
        id: WindowId,
        window: Weak<Window>,
        chrome: WindowChrome,
        devtools: DevToolsHandle,
    ) -> Self {
        Self {
            id,
            window,
            chrome,
            devtools,
        }
    }

    /// Returns the stable identity assigned when the window was opened.
    pub const fn id(&self) -> WindowId {
        self.id
    }

    /// Returns whether the application runtime still owns the native window.
    pub fn is_open(&self) -> bool {
        self.window.strong_count() > 0
    }

    /// Captures current platform state or reports that the window has closed.
    pub fn state(&self) -> Result<WindowState, WindowOperationError> {
        let window = self.live_window("window state query")?;
        Ok(WindowState::from_native(&window))
    }

    /// Schedules a redraw or reports that the window has closed.
    pub fn request_redraw(&self) -> Result<(), WindowOperationError> {
        self.live_window("redraw request")?.request_redraw();
        Ok(())
    }

    /// Returns the shared DevTools session capability for this window.
    pub fn devtools(&self) -> DevToolsHandle {
        self.devtools.clone()
    }

    /// Opens the default zui DevTools window for this window and schedules a frame.
    pub fn open_devtools(&self) -> Result<(), WindowOperationError> {
        let window = self.live_window("open DevTools")?;
        self.devtools.open();
        window.request_redraw();
        Ok(())
    }

    /// Closes the default zui DevTools window for this window and schedules a frame.
    pub fn close_devtools(&self) -> Result<(), WindowOperationError> {
        let window = self.live_window("close DevTools")?;
        self.devtools.close();
        window.request_redraw();
        Ok(())
    }

    /// Toggles DevTools for this window and returns whether it is now open.
    pub fn toggle_devtools(&self) -> Result<bool, WindowOperationError> {
        let window = self.live_window("toggle DevTools")?;
        let is_open = self.devtools.toggle();
        window.request_redraw();
        Ok(is_open)
    }

    /// Returns whether DevTools is currently open for this window.
    pub fn is_devtools_open(&self) -> bool {
        self.devtools.is_open()
    }

    /// Begins a platform window drag when the runtime still owns the window.
    pub fn start_window_drag(&self) -> Result<(), WindowOperationError> {
        self.live_window("window drag")?
            .drag_window()
            .map_err(|source| WindowOperationError::Platform {
                window: self.id,
                operation: "window drag",
                source: Box::new(source),
            })?;
        Ok(())
    }

    /// Updates the pointer cursor or reports that the window has closed.
    pub fn set_cursor(&self, cursor: CursorIcon) -> Result<(), WindowOperationError> {
        self.live_window("cursor update")?
            .set_cursor(cursor.into_native());
        Ok(())
    }

    /// Updates the platform window title or reports that the window has closed.
    pub fn set_title(&self, title: &str) -> Result<(), WindowOperationError> {
        self.live_window("title update")?.set_title(title);
        Ok(())
    }

    /// Requests a new logical inner size and returns an immediately applied physical extent.
    pub fn request_inner_logical_size(
        &self,
        size: LogicalSize,
    ) -> Result<Option<PhysicalExtent>, WindowOperationError> {
        if !size.is_valid() {
            return Err(WindowOperationError::InvalidSize {
                window: self.id,
                operation: "inner size request",
            });
        }
        Ok(self
            .live_window("inner size request")?
            .request_inner_size(size.into_native())
            .map(|size| PhysicalExtent::new(size.width, size.height)))
    }

    /// Returns the current platform theme preference or reports that the window has closed.
    pub fn theme(&self) -> Result<Option<Theme>, WindowOperationError> {
        Ok(self
            .live_window("theme query")?
            .theme()
            .map(Theme::from_native))
    }

    /// Applies an explicit platform theme or reports that the window has closed.
    pub fn set_theme(&self, theme: Option<Theme>) -> Result<(), WindowOperationError> {
        self.live_window("theme update")?
            .set_theme(theme.map(Theme::into_native));
        Ok(())
    }

    /// Shows or hides the native window.
    pub fn set_visible(&self, visible: bool) -> Result<(), WindowOperationError> {
        self.live_window("visibility update")?.set_visible(visible);
        Ok(())
    }

    /// Requests keyboard focus for the native window.
    pub fn focus(&self) -> Result<(), WindowOperationError> {
        self.live_window("focus request")?.focus_window();
        Ok(())
    }

    /// Changes the platform minimization state.
    pub fn set_minimized(&self, minimized: bool) -> Result<(), WindowOperationError> {
        self.live_window("minimization update")?
            .set_minimized(minimized);
        Ok(())
    }

    /// Changes the platform maximization state.
    pub fn set_maximized(&self, maximized: bool) -> Result<(), WindowOperationError> {
        self.live_window("maximization update")?
            .set_maximized(maximized);
        Ok(())
    }

    /// Enters or leaves borderless fullscreen mode.
    pub fn set_fullscreen(&self, fullscreen: bool) -> Result<(), WindowOperationError> {
        self.live_window("fullscreen update")?
            .set_fullscreen(fullscreen.then_some(winit::window::Fullscreen::Borderless(None)));
        Ok(())
    }

    /// Changes whether the user can resize the native window.
    pub fn set_resizable(&self, resizable: bool) -> Result<(), WindowOperationError> {
        self.live_window("resizability update")?
            .set_resizable(resizable);
        Ok(())
    }

    /// Enables platform text input or reports that the window has closed.
    pub fn enable_ime(&self) -> Result<(), WindowOperationError> {
        self.live_window("IME enable")?.set_ime_allowed(true);
        Ok(())
    }

    /// Disables platform text input or reports that the window has closed.
    pub fn disable_ime(&self) -> Result<(), WindowOperationError> {
        self.live_window("IME disable")?.set_ime_allowed(false);
        Ok(())
    }

    /// Updates the IME candidate area or reports that the window has closed.
    pub fn set_ime_cursor_area(&self, area: ImeCursorArea) -> Result<(), WindowOperationError> {
        self.live_window("IME cursor-area update")?
            .set_ime_cursor_area(
                LogicalPosition::new(area.x, area.y),
                winit::dpi::LogicalSize::new(area.width, area.height),
            );
        Ok(())
    }

    /// Returns logical insets occupied by native controls for this window's chrome policy.
    pub fn window_control_insets(&self) -> WindowControlInsets {
        window_control_insets(self.chrome)
    }

    fn live_window(&self, operation: &'static str) -> Result<Arc<Window>, WindowOperationError> {
        self.window.upgrade().ok_or(WindowOperationError::Closed {
            window: self.id,
            operation,
        })
    }

    #[cfg(target_os = "windows")]
    pub(crate) fn native_hwnd(&self) -> Option<isize> {
        use raw_window_handle::HasWindowHandle;

        let window = self.window.upgrade()?;
        let handle = window.window_handle().ok()?;
        match handle.as_raw() {
            raw_window_handle::RawWindowHandle::Win32(handle) => Some(handle.hwnd.get()),
            _ => None,
        }
    }
}

#[cfg(test)]
#[path = "capability_tests.rs"]
mod tests;
