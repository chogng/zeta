use crate::window::PhysicalExtent;
use crate::window::WindowId;

use super::DiagnosticEventKind;
use super::DiagnosticsHandle;
use super::WindowMetrics;

#[test]
fn bounded_trace_retains_latest_events_and_live_windows() {
    let diagnostics = DiagnosticsHandle::new(2, None);
    let window = WindowId::from_raw(7);
    diagnostics.record(DiagnosticEventKind::Resumed);
    diagnostics.open_window(
        window,
        WindowMetrics::new(PhysicalExtent::new(800, 600), 2.0),
    );
    diagnostics.record(DiagnosticEventKind::WindowEvent(window));
    diagnostics.set_work_counts(3, 2);

    let snapshot = diagnostics.snapshot();
    assert_eq!(snapshot.events.len(), 2);
    assert_eq!(snapshot.events[0].sequence, 2);
    assert_eq!(snapshot.windows.len(), 1);
    assert_eq!(snapshot.active_tasks, 3);
    assert_eq!(snapshot.active_timers, 2);
}
