use std::collections::HashMap;
use std::collections::VecDeque;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::Duration;
use std::time::Instant;

use crate::render::RenderOutcome;
use crate::ui::presentation::InspectionFrame;
use crate::ui::presentation::UiScene;
use crate::window::WindowId;
use crate::window::WindowMetrics;

mod inspection;

pub use inspection::DevToolsHandle;
pub use inspection::InspectionSelection;
pub use inspection::InspectorState;

/// Runtime transition captured by the bounded ZUI diagnostic trace.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DiagnosticEventKind {
    Resumed,
    Suspended,
    WindowOpened(WindowId),
    WindowEvent(WindowId),
    FramePresented {
        window: WindowId,
        outcome: RenderOutcome,
    },
    WindowClosed(WindowId),
    UserEvent,
    MenuAction,
    TrayEvent,
    GlobalShortcut,
    OpenUrl,
    AccessibilityAction,
    Exiting,
}

/// Sequenced runtime trace entry with monotonic time since application construction.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DiagnosticEvent {
    pub sequence: u64,
    pub elapsed: Duration,
    pub kind: DiagnosticEventKind,
}

/// Structural summary of the most recently submitted UI scene.
///
/// When diagnostics inspection retention is enabled, `inspection` contains a copy of the
/// scene's complete per-frame inspection hierarchy. It is omitted by default so applications
/// that only need counters do not copy every inspection node on every frame.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SceneDiagnostics {
    pub rectangles: usize,
    pub icons: usize,
    pub images: usize,
    pub text_blocks: usize,
    pub batches: usize,
    pub accessibility_nodes: usize,
    pub inspection: Option<InspectionFrame>,
}

impl SceneDiagnostics {
    pub(crate) fn from_scene(
        scene: &UiScene,
        accessibility_nodes: usize,
        retain_inspection: bool,
    ) -> Self {
        Self {
            rectangles: scene.rects().len(),
            icons: scene.icons().len(),
            images: scene.images().len(),
            text_blocks: scene.text_blocks().len(),
            batches: scene.batches().count(),
            accessibility_nodes,
            inspection: retain_inspection.then(|| scene.inspection().clone()),
        }
    }
}

/// Latest observable state for one runtime-owned window.
#[derive(Clone, Debug, PartialEq)]
pub struct WindowDiagnostics {
    pub id: WindowId,
    pub metrics: WindowMetrics,
    pub presented_frames: u64,
    pub last_scene: Option<SceneDiagnostics>,
}

/// Immutable snapshot consumed by diagnostics UI, tests, or support tooling.
#[derive(Clone, Debug, PartialEq)]
pub struct DiagnosticsSnapshot {
    pub uptime: Duration,
    pub windows: Vec<WindowDiagnostics>,
    pub active_tasks: usize,
    pub active_timers: usize,
    pub events: Vec<DiagnosticEvent>,
}

/// Optional streaming observer invoked after an event enters the bounded trace.
pub trait DiagnosticsSink: Send + Sync {
    /// Observes one immutable diagnostic event.
    fn record(&self, event: &DiagnosticEvent);
}

#[derive(Clone)]
pub struct DiagnosticsHandle {
    retain_inspection: bool,
    state: Arc<Mutex<DiagnosticsState>>,
}

impl DiagnosticsHandle {
    pub(crate) fn new(
        capacity: usize,
        sink: Option<Arc<dyn DiagnosticsSink>>,
        retain_inspection: bool,
    ) -> Self {
        Self {
            retain_inspection,
            state: Arc::new(Mutex::new(DiagnosticsState {
                started: Instant::now(),
                capacity,
                next_sequence: 1,
                events: VecDeque::with_capacity(capacity),
                windows: HashMap::new(),
                active_tasks: 0,
                active_timers: 0,
                sink,
            })),
        }
    }

    /// Captures current runtime state and a stable copy of the bounded trace.
    pub fn snapshot(&self) -> DiagnosticsSnapshot {
        let state = self.state.lock().expect("diagnostics lock");
        let mut windows = state.windows.values().cloned().collect::<Vec<_>>();
        windows.sort_by_key(|window| window.id.into_raw());
        DiagnosticsSnapshot {
            uptime: state.started.elapsed(),
            windows,
            active_tasks: state.active_tasks,
            active_timers: state.active_timers,
            events: state.events.iter().cloned().collect(),
        }
    }

    /// Removes trace history without changing live runtime state.
    pub fn clear_events(&self) {
        self.state.lock().expect("diagnostics lock").events.clear();
    }

    pub(crate) fn scene_diagnostics(
        &self,
        scene: &UiScene,
        accessibility_nodes: usize,
    ) -> SceneDiagnostics {
        SceneDiagnostics::from_scene(scene, accessibility_nodes, self.retain_inspection)
    }

    pub(crate) fn record(&self, kind: DiagnosticEventKind) {
        let (event, sink) = {
            let mut state = self.state.lock().expect("diagnostics lock");
            let event = DiagnosticEvent {
                sequence: state.next_sequence,
                elapsed: state.started.elapsed(),
                kind,
            };
            state.next_sequence += 1;
            if state.capacity > 0 {
                while state.events.len() >= state.capacity {
                    state.events.pop_front();
                }
                state.events.push_back(event.clone());
            }
            (event, state.sink.clone())
        };
        if let Some(sink) = sink {
            sink.record(&event);
        }
    }

    pub(crate) fn open_window(&self, id: WindowId, metrics: WindowMetrics) {
        self.state.lock().expect("diagnostics lock").windows.insert(
            id,
            WindowDiagnostics {
                id,
                metrics,
                presented_frames: 0,
                last_scene: None,
            },
        );
        self.record(DiagnosticEventKind::WindowOpened(id));
    }

    pub(crate) fn update_window(&self, id: WindowId, metrics: WindowMetrics) {
        if let Some(window) = self
            .state
            .lock()
            .expect("diagnostics lock")
            .windows
            .get_mut(&id)
        {
            window.metrics = metrics;
        }
    }

    pub(crate) fn present(
        &self,
        id: WindowId,
        metrics: WindowMetrics,
        scene: SceneDiagnostics,
        outcome: RenderOutcome,
    ) {
        if let Some(window) = self
            .state
            .lock()
            .expect("diagnostics lock")
            .windows
            .get_mut(&id)
        {
            window.metrics = metrics;
            window.presented_frames += 1;
            window.last_scene = Some(scene);
        }
        self.record(DiagnosticEventKind::FramePresented {
            window: id,
            outcome,
        });
    }

    pub(crate) fn close_window(&self, id: WindowId) {
        self.state
            .lock()
            .expect("diagnostics lock")
            .windows
            .remove(&id);
        self.record(DiagnosticEventKind::WindowClosed(id));
    }

    pub(crate) fn set_work_counts(&self, active_tasks: usize, active_timers: usize) {
        let mut state = self.state.lock().expect("diagnostics lock");
        state.active_tasks = active_tasks;
        state.active_timers = active_timers;
    }
}

struct DiagnosticsState {
    started: Instant,
    capacity: usize,
    next_sequence: u64,
    events: VecDeque<DiagnosticEvent>,
    windows: HashMap<WindowId, WindowDiagnostics>,
    active_tasks: usize,
    active_timers: usize,
    sink: Option<Arc<dyn DiagnosticsSink>>,
}

#[cfg(test)]
#[path = "devtools/tests.rs"]
mod tests;
