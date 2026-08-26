use std::sync::Weak;

use crate::devtools::DevToolsHandle;

use super::LogicalSize;
use super::WindowChrome;
use super::WindowHandle;
use super::WindowId;
use super::WindowOperationError;

fn closed_handle(id: WindowId) -> WindowHandle {
    WindowHandle::new(id, Weak::new(), WindowChrome::Native, DevToolsHandle::new())
}

#[test]
fn closed_handles_keep_their_identity_and_report_failed_operations() {
    let id = WindowId::from_raw(42);
    let handle = closed_handle(id);

    assert_eq!(handle.id(), id);
    assert!(!handle.is_open());
    assert!(matches!(
        handle.request_redraw(),
        Err(WindowOperationError::Closed { window, .. }) if window == id
    ));
    assert!(handle.state().unwrap_err().is_closed());
}

#[test]
fn invalid_live_operation_input_is_distinct_from_a_closed_window() {
    let id = WindowId::from_raw(7);
    let handle = closed_handle(id);

    assert!(matches!(
        handle.request_inner_logical_size(LogicalSize::new(f64::NAN, 10.0)),
        Err(WindowOperationError::InvalidSize { window, .. }) if window == id
    ));
}
