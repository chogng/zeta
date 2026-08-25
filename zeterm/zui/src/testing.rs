//! Deterministic application and renderer tools for framework consumer tests.

use std::collections::BTreeMap;
use std::collections::VecDeque;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::Duration;
use std::time::Instant;

use crate::app::ExitPolicy;
use crate::app::ProtocolUrl;
use crate::render::RenderOutcome;
use crate::render::RenderTargetSize;
use crate::render::Renderer;
use crate::render::RendererError;
use crate::runtime::AccessibilityNode;
use crate::services::GlobalShortcutEvent;
use crate::services::TrayEvent;
use crate::ui::presentation::UiScene;
use crate::window::LogicalSize;
use crate::window::WindowId;

/// Manually advanced monotonic clock used by deterministic application tests.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TestClock {
    now: Instant,
}

impl TestClock {
    /// Creates a clock at an explicit monotonic instant.
    pub const fn at(now: Instant) -> Self {
        Self { now }
    }

    /// Returns the current test-controlled instant.
    pub const fn now(self) -> Instant {
        self.now
    }

    /// Advances the clock without sleeping.
    pub fn advance(&mut self, duration: Duration) {
        self.now = self
            .now
            .checked_add(duration)
            .expect("test clock advance must remain within the Instant range");
    }
}

/// Observable state shared by clones of a [`HeadlessRenderer`].
#[derive(Clone, Debug, PartialEq)]
pub struct HeadlessRenderState {
    target_size: RenderTargetSize,
    scale_factor: f64,
    blank_frame_count: usize,
    scenes: Vec<UiScene>,
}

impl Default for HeadlessRenderState {
    fn default() -> Self {
        Self {
            target_size: RenderTargetSize::new(0, 0),
            scale_factor: 1.0,
            blank_frame_count: 0,
            scenes: Vec::new(),
        }
    }
}

impl HeadlessRenderState {
    /// Returns the last physical target size supplied by the runtime.
    pub const fn target_size(&self) -> RenderTargetSize {
        self.target_size
    }

    /// Returns the last logical-to-physical scale factor.
    pub const fn scale_factor(&self) -> f64 {
        self.scale_factor
    }

    /// Returns the number of renderer calls without scene content.
    pub const fn blank_frame_count(&self) -> usize {
        self.blank_frame_count
    }

    /// Returns every immutable scene submitted in presentation order.
    pub fn scenes(&self) -> &[UiScene] {
        &self.scenes
    }
}

/// Cloneable renderer that records immutable scenes without a graphics device or native surface.
#[derive(Clone, Default)]
pub struct HeadlessRenderer {
    state: Arc<Mutex<HeadlessRenderState>>,
}

impl HeadlessRenderer {
    /// Returns a snapshot of all renderer state recorded so far.
    pub fn state(&self) -> HeadlessRenderState {
        self.state.lock().expect("headless renderer lock").clone()
    }

    /// Removes all recorded scenes while preserving target configuration and frame counts.
    pub fn clear_scenes(&self) {
        self.state
            .lock()
            .expect("headless renderer lock")
            .scenes
            .clear();
    }
}

impl Renderer for HeadlessRenderer {
    fn resize(&mut self, size: RenderTargetSize) {
        self.state
            .lock()
            .expect("headless renderer lock")
            .target_size = size;
    }

    fn set_scale_factor(&mut self, scale_factor: f64) {
        self.state
            .lock()
            .expect("headless renderer lock")
            .scale_factor = scale_factor;
    }

    fn render(&mut self) -> Result<RenderOutcome, RendererError> {
        self.state
            .lock()
            .expect("headless renderer lock")
            .blank_frame_count += 1;
        Ok(RenderOutcome::Presented)
    }

    fn render_scene(&mut self, scene: &UiScene) -> Result<RenderOutcome, RendererError> {
        self.state
            .lock()
            .expect("headless renderer lock")
            .scenes
            .push(scene.clone());
        Ok(RenderOutcome::Presented)
    }
}

/// Scope used to cancel deterministic timers with a test-owned window.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TestTimerScope {
    Application,
    Window(WindowId),
}

/// Stable identity of one deterministic timer.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct TestTimerId(u64);

/// Runtime event emitted by [`TestRuntime`] in deterministic FIFO order.
#[derive(Clone, Debug, PartialEq)]
pub enum TestEvent<T> {
    Resumed,
    WindowOpened(WindowId),
    RedrawRequested(WindowId),
    User(T),
    Tray(TrayEvent),
    GlobalShortcut(GlobalShortcutEvent),
    OpenUrl(ProtocolUrl),
    WindowClosed(WindowId),
    Exiting,
}

/// Inspectable headless window owned by a deterministic test runtime.
pub struct TestWindow {
    id: WindowId,
    title: String,
    logical_size: LogicalSize,
    renderer: HeadlessRenderer,
    accessibility: Vec<AccessibilityNode>,
}

impl TestWindow {
    /// Returns the stable test window identity.
    pub const fn id(&self) -> WindowId {
        self.id
    }

    /// Returns the configured title.
    pub fn title(&self) -> &str {
        &self.title
    }

    /// Returns the configured logical size.
    pub const fn logical_size(&self) -> LogicalSize {
        self.logical_size
    }

    /// Returns a shared handle to this window's headless renderer.
    pub fn renderer(&self) -> HeadlessRenderer {
        self.renderer.clone()
    }

    /// Returns the most recently presented accessibility snapshot.
    pub fn accessibility(&self) -> &[AccessibilityNode] {
        &self.accessibility
    }
}

struct TestTimer<T> {
    scope: TestTimerScope,
    event: T,
}

/// Deterministic lifecycle, event, timer, and presentation host for application tests.
pub struct TestRuntime<T> {
    clock: TestClock,
    exit_policy: ExitPolicy,
    next_window: u64,
    next_timer: u64,
    windows: BTreeMap<u64, TestWindow>,
    timers: BTreeMap<(Instant, TestTimerId), TestTimer<T>>,
    events: VecDeque<TestEvent<T>>,
    exiting: bool,
}

impl<T> TestRuntime<T> {
    /// Creates an empty runtime at an explicit monotonic instant.
    pub fn at(now: Instant) -> Self {
        Self {
            clock: TestClock::at(now),
            exit_policy: ExitPolicy::default(),
            next_window: 1,
            next_timer: 1,
            windows: BTreeMap::new(),
            timers: BTreeMap::new(),
            events: VecDeque::new(),
            exiting: false,
        }
    }

    /// Selects the same last-window exit policy used by native applications.
    pub const fn with_exit_policy(mut self, exit_policy: ExitPolicy) -> Self {
        self.exit_policy = exit_policy;
        self
    }

    /// Emits the initial resume callback event.
    pub fn resume(&mut self) {
        self.events.push_back(TestEvent::Resumed);
    }

    /// Opens one headless window and emits its lifecycle event.
    pub fn open_window(&mut self, title: impl Into<String>, logical_size: LogicalSize) -> WindowId {
        let raw = self.next_window;
        self.next_window += 1;
        let id = WindowId::from_raw(raw);
        self.windows.insert(
            raw,
            TestWindow {
                id,
                title: title.into(),
                logical_size,
                renderer: HeadlessRenderer::default(),
                accessibility: Vec::new(),
            },
        );
        self.events.push_back(TestEvent::WindowOpened(id));
        id
    }

    /// Returns one live headless window.
    pub fn window(&self, id: WindowId) -> Option<&TestWindow> {
        self.windows.get(&id.into_raw())
    }

    /// Requests a redraw for a live window.
    pub fn request_redraw(&mut self, id: WindowId) {
        if self.windows.contains_key(&id.into_raw()) {
            self.events.push_back(TestEvent::RedrawRequested(id));
        }
    }

    /// Presents a scene and accessibility snapshot into a live headless window.
    pub fn present_scene(
        &mut self,
        id: WindowId,
        scene: &UiScene,
        accessibility: &[AccessibilityNode],
    ) -> Result<Option<RenderOutcome>, RendererError> {
        let Some(window) = self.windows.get_mut(&id.into_raw()) else {
            return Ok(None);
        };
        window.accessibility = accessibility.to_vec();
        window.renderer.render_scene(scene).map(Some)
    }

    /// Enqueues an application-defined event immediately.
    pub fn send_event(&mut self, event: T) {
        self.events.push_back(TestEvent::User(event));
    }

    /// Enqueues one platform-independent tray interaction.
    pub fn send_tray_event(&mut self, event: TrayEvent) {
        self.events.push_back(TestEvent::Tray(event));
    }

    /// Enqueues one platform-independent global shortcut interaction.
    pub fn send_global_shortcut(&mut self, event: GlobalShortcutEvent) {
        self.events.push_back(TestEvent::GlobalShortcut(event));
    }

    /// Enqueues one application protocol URL.
    pub fn send_open_url(&mut self, url: ProtocolUrl) {
        self.events.push_back(TestEvent::OpenUrl(url));
    }

    /// Schedules an event relative to the manual clock.
    pub fn schedule_after(
        &mut self,
        scope: TestTimerScope,
        delay: Duration,
        event: T,
    ) -> TestTimerId {
        let id = TestTimerId(self.next_timer);
        self.next_timer += 1;
        let deadline = self
            .clock
            .now()
            .checked_add(delay)
            .expect("test timer deadline must remain within the Instant range");
        self.timers
            .insert((deadline, id), TestTimer { scope, event });
        id
    }

    /// Cancels one deterministic timer if it is still pending.
    pub fn cancel_timer(&mut self, id: TestTimerId) {
        self.timers.retain(|(_, candidate), _| *candidate != id);
    }

    /// Advances the manual clock and enqueues every due timer in stable deadline order.
    pub fn advance(&mut self, duration: Duration) {
        self.clock.advance(duration);
        let due = self
            .timers
            .range(..=(self.clock.now(), TestTimerId(u64::MAX)))
            .map(|(key, _)| *key)
            .collect::<Vec<_>>();
        for key in due {
            if let Some(timer) = self.timers.remove(&key) {
                self.events.push_back(TestEvent::User(timer.event));
            }
        }
    }

    /// Closes one window, cancelling its timers and applying the configured exit policy.
    pub fn close_window(&mut self, id: WindowId) {
        if self.windows.remove(&id.into_raw()).is_none() {
            return;
        }
        self.timers
            .retain(|_, timer| timer.scope != TestTimerScope::Window(id));
        self.events.push_back(TestEvent::WindowClosed(id));
        if self.windows.is_empty() && self.exit_policy == ExitPolicy::OnLastWindowClosed {
            self.exit();
        }
    }

    /// Emits one exit event and cancels every pending timer.
    pub fn exit(&mut self) {
        if self.exiting {
            return;
        }
        self.exiting = true;
        self.timers.clear();
        self.events.push_back(TestEvent::Exiting);
    }

    /// Removes the next deterministic runtime event.
    pub fn next_event(&mut self) -> Option<TestEvent<T>> {
        self.events.pop_front()
    }
}

#[cfg(test)]
#[path = "testing/testing_tests.rs"]
mod tests;
