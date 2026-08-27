//! Diagnostics for product session switches and terminal adoption.

use std::fmt::Display;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

static ENABLED: OnceLock<bool> = OnceLock::new();
static FRAMES_ENABLED: OnceLock<bool> = OnceLock::new();
static NEXT_SWITCH_ID: AtomicU64 = AtomicU64::new(1);

/// Returns whether opt-in Session Tab diagnostics are enabled for this process.
pub(crate) fn enabled() -> bool {
    *ENABLED.get_or_init(|| {
        matches!(
            std::env::var("APP_SESSION_TRACE").as_deref(),
            Ok("1") | Ok("true") | Ok("yes")
        )
    })
}

/// Returns whether per-frame redraw and renderer timing is explicitly requested.
pub(crate) fn frames_enabled() -> bool {
    enabled()
        && *FRAMES_ENABLED.get_or_init(|| {
            matches!(
                std::env::var("APP_SESSION_TRACE_FRAMES").as_deref(),
                Ok("1") | Ok("true") | Ok("yes")
            )
        })
}

/// Correlates one product-owned Session Tab activation across the UI and Agent worker threads.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct SwitchId(u64);

impl SwitchId {
    pub(crate) const fn new(value: u64) -> Self {
        Self(value)
    }

    pub(crate) fn next() -> Self {
        Self(NEXT_SWITCH_ID.fetch_add(1, Ordering::Relaxed))
    }

    pub(crate) const fn get(self) -> u64 {
        self.0
    }
}

/// Emits one opt-in diagnostic event without introducing a logging dependency into the native
/// host. The event is deliberately scoped to Session Tab diagnosis rather than becoming a second
/// application-wide telemetry runtime.
pub(crate) fn event(id: Option<SwitchId>, label: &'static str, details: impl Display) {
    if !enabled() {
        return;
    }
    match id {
        Some(id) => eprintln!(
            "[app session-trace] switch={} event={} {}",
            id.0, label, details
        ),
        None => eprintln!("[app session-trace] event={} {}", label, details),
    }
}

/// Measures one synchronous phase when Session Tab diagnostics are enabled.
pub(crate) struct Span {
    id: Option<SwitchId>,
    label: &'static str,
    started: Instant,
    enabled: bool,
}

impl Span {
    pub(crate) fn new(id: Option<SwitchId>, label: &'static str) -> Self {
        Self::with_enabled(id, label, enabled())
    }

    pub(crate) fn frame(label: &'static str) -> Self {
        Self::with_enabled(None, label, frames_enabled())
    }

    fn with_enabled(id: Option<SwitchId>, label: &'static str, enabled: bool) -> Self {
        Self {
            id,
            label,
            started: Instant::now(),
            enabled,
        }
    }
}

impl Drop for Span {
    fn drop(&mut self) {
        if !self.enabled {
            return;
        }
        let elapsed = self.started.elapsed();
        match self.id {
            Some(id) => eprintln!(
                "[app session-trace] switch={} phase={} elapsed_us={}",
                id.0,
                self.label,
                elapsed.as_micros()
            ),
            None => eprintln!(
                "[app session-trace] phase={} elapsed_us={}",
                self.label,
                elapsed.as_micros()
            ),
        }
    }
}
