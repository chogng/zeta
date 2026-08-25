use std::cell::RefCell;
use std::fmt::Display;
use std::rc::Rc;

use thiserror::Error;

/// Failure while reading or writing text through a platform clipboard capability.
#[derive(Clone, Debug, Error, Eq, PartialEq)]
pub enum ClipboardError {
    /// The current target has no system clipboard implementation.
    #[error("system clipboard is unsupported on this platform")]
    Unsupported,
    /// The platform clipboard rejected an operation.
    #[error("system clipboard {operation} failed: {message}")]
    Backend {
        /// Stable operation label.
        operation: &'static str,
        /// Backend diagnostic without exposing backend types in the public contract.
        message: String,
    },
}

impl ClipboardError {
    #[cfg(not(target_os = "android"))]
    fn backend(operation: &'static str, error: impl Display) -> Self {
        Self::Backend {
            operation,
            message: error.to_string(),
        }
    }
}

/// Main-thread text clipboard capability supplied to an application.
///
/// Platform implementations own backend handles. Products consume this small contract and do not
/// construct a second clipboard backend or depend on platform clipboard libraries directly.
pub trait Clipboard {
    /// Replaces the system clipboard text.
    fn write_text(&mut self, text: String) -> Result<(), ClipboardError>;

    /// Reads the current system clipboard text.
    fn read_text(&mut self) -> Result<String, ClipboardError>;
}

/// Cloneable main-thread handle to the runtime-owned clipboard capability.
#[derive(Clone)]
pub struct ClipboardHandle {
    clipboard: Rc<RefCell<Box<dyn Clipboard>>>,
}

impl ClipboardHandle {
    pub(crate) fn new(clipboard: impl Clipboard + 'static) -> Self {
        Self {
            clipboard: Rc::new(RefCell::new(Box::new(clipboard))),
        }
    }

    /// Replaces the platform clipboard text.
    pub fn write_text(&self, text: String) -> Result<(), ClipboardError> {
        self.clipboard.borrow_mut().write_text(text)
    }

    /// Reads the current platform clipboard text.
    pub fn read_text(&self) -> Result<String, ClipboardError> {
        self.clipboard.borrow_mut().read_text()
    }
}

/// Default platform text clipboard implementation.
#[derive(Clone, Copy, Debug, Default)]
pub struct SystemClipboard;

#[cfg(not(target_os = "android"))]
impl Clipboard for SystemClipboard {
    fn write_text(&mut self, text: String) -> Result<(), ClipboardError> {
        let mut clipboard = arboard::Clipboard::new()
            .map_err(|error| ClipboardError::backend("initialization", error))?;
        clipboard
            .set_text(text)
            .map_err(|error| ClipboardError::backend("write", error))
    }

    fn read_text(&mut self) -> Result<String, ClipboardError> {
        let mut clipboard = arboard::Clipboard::new()
            .map_err(|error| ClipboardError::backend("initialization", error))?;
        clipboard
            .get_text()
            .map_err(|error| ClipboardError::backend("read", error))
    }
}

#[cfg(target_os = "android")]
impl Clipboard for SystemClipboard {
    fn write_text(&mut self, _text: String) -> Result<(), ClipboardError> {
        Err(ClipboardError::Unsupported)
    }

    fn read_text(&mut self) -> Result<String, ClipboardError> {
        Err(ClipboardError::Unsupported)
    }
}

#[cfg(test)]
#[path = "clipboard_tests.rs"]
mod tests;
