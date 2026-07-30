use std::time::{Duration, Instant};

const DEFAULT_HOLD_DURATION: Duration = Duration::from_millis(700);
const DEFAULT_FADE_IN_DURATION: Duration = Duration::from_millis(120);
const DEFAULT_FADE_OUT_DURATION: Duration = Duration::from_millis(220);
const ANIMATION_FRAME_INTERVAL: Duration = Duration::from_millis(16);

/// Pointer relationship used to project scrollbar hover without ambiguous boolean arguments.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum ScrollbarPointerPresence {
    #[default]
    Outside,
    Over,
}

/// Interaction state used by [`super::ScrollbarStyle`] to select semantic colors.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum ScrollbarState {
    #[default]
    Resting,
    Hovered,
    Active,
}

/// Current visual state supplied to a [`super::ScrollView`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScrollbarPresentation {
    state: ScrollbarState,
    opacity: f32,
}

impl ScrollbarPresentation {
    pub const fn new(state: ScrollbarState, opacity: f32) -> Self {
        Self { state, opacity }
    }

    pub const fn state(self) -> ScrollbarState {
        self.state
    }

    pub fn opacity(self) -> f32 {
        self.opacity.clamp(0.0, 1.0)
    }
}

impl Default for ScrollbarPresentation {
    fn default() -> Self {
        Self::new(ScrollbarState::Resting, 1.0)
    }
}

#[derive(Clone, Copy, Debug)]
struct OpacityTransition {
    started_at: Instant,
    duration: Duration,
    from: f32,
    to: f32,
}

impl OpacityTransition {
    fn opacity_at(self, now: Instant) -> f32 {
        if now <= self.started_at {
            return self.from;
        }
        let progress =
            now.duration_since(self.started_at).as_secs_f32() / self.duration.as_secs_f32();
        self.from + (self.to - self.from) * progress.clamp(0.0, 1.0)
    }

    fn ends_at(self) -> Instant {
        self.started_at
            .checked_add(self.duration)
            .unwrap_or(self.started_at)
    }
}

/// Platform-independent hover, active, and fade state for one scrollbar.
///
/// The controller does not install timers or read pointer events. Hosts project pointer presence,
/// call [`Self::activity`] after wheel/keyboard scrolling, retain drag capture separately, and
/// schedule [`Self::advance`] using [`Self::next_deadline`].
#[derive(Clone, Debug)]
pub struct ScrollbarController {
    hold_duration: Duration,
    fade_in_duration: Duration,
    fade_out_duration: Duration,
    state: ScrollbarState,
    opacity: f32,
    transition: Option<OpacityTransition>,
    visible_until: Option<Instant>,
    next_deadline: Option<Instant>,
}

impl ScrollbarController {
    /// Creates an initially hidden controller with explicit animation timing.
    ///
    /// # Panics
    ///
    /// Panics when either fade duration is zero.
    pub fn new(
        hold_duration: Duration,
        fade_in_duration: Duration,
        fade_out_duration: Duration,
    ) -> Self {
        assert!(
            !fade_in_duration.is_zero() && !fade_out_duration.is_zero(),
            "scrollbar fade durations must be non-zero"
        );
        Self {
            hold_duration,
            fade_in_duration,
            fade_out_duration,
            state: ScrollbarState::Resting,
            opacity: 0.0,
            transition: None,
            visible_until: None,
            next_deadline: None,
        }
    }

    pub fn presentation(&self) -> ScrollbarPresentation {
        ScrollbarPresentation::new(self.state, self.opacity)
    }

    pub const fn next_deadline(&self) -> Option<Instant> {
        self.next_deadline
    }

    /// Reveals the scrollbar and extends its resting hold period after scroll activity.
    pub fn activity(&mut self, now: Instant) {
        self.resolve_transition(now);
        self.visible_until = now.checked_add(self.hold_duration);
        self.transition_to(1.0, self.fade_in_duration, now);
    }

    /// Projects whether the pointer is currently over the scrollbar track.
    pub fn pointer_presence(&mut self, presence: ScrollbarPointerPresence, now: Instant) {
        self.resolve_transition(now);
        match presence {
            ScrollbarPointerPresence::Outside => {
                if self.state == ScrollbarState::Hovered {
                    self.state = ScrollbarState::Resting;
                    self.visible_until = now.checked_add(self.hold_duration);
                    self.schedule_resting(now);
                }
            }
            ScrollbarPointerPresence::Over => {
                if self.state != ScrollbarState::Active {
                    self.state = ScrollbarState::Hovered;
                    self.visible_until = None;
                    self.transition_to(1.0, self.fade_in_duration, now);
                }
            }
        }
    }

    /// Keeps the scrollbar visible with active colors while its thumb owns pointer capture.
    pub fn begin_drag(&mut self, now: Instant) {
        self.resolve_transition(now);
        self.state = ScrollbarState::Active;
        self.visible_until = None;
        self.transition_to(1.0, self.fade_in_duration, now);
    }

    /// Ends pointer capture and projects the pointer's current relationship to the track.
    pub fn end_drag(&mut self, presence: ScrollbarPointerPresence, now: Instant) {
        self.resolve_transition(now);
        match presence {
            ScrollbarPointerPresence::Over => {
                self.state = ScrollbarState::Hovered;
                self.visible_until = None;
                self.transition_to(1.0, self.fade_in_duration, now);
            }
            ScrollbarPointerPresence::Outside => {
                self.state = ScrollbarState::Resting;
                self.visible_until = now.checked_add(self.hold_duration);
                self.schedule_resting(now);
            }
        }
    }

    /// Cancels hover, capture, and animation when the owning window becomes inactive.
    pub fn cancel(&mut self) {
        self.state = ScrollbarState::Resting;
        self.opacity = 0.0;
        self.transition = None;
        self.visible_until = None;
        self.next_deadline = None;
    }

    /// Advances an in-flight fade and returns whether the painted presentation changed.
    pub fn advance(&mut self, now: Instant) -> bool {
        let previous = self.presentation();
        self.resolve_transition(now);
        if self.transition.is_none()
            && self.state == ScrollbarState::Resting
            && self
                .visible_until
                .is_some_and(|visible_until| now >= visible_until)
        {
            self.visible_until = None;
            self.transition_to(0.0, self.fade_out_duration, now);
            self.resolve_transition(now);
        }
        self.schedule_resting(now);
        self.presentation() != previous
    }

    fn resolve_transition(&mut self, now: Instant) {
        let Some(transition) = self.transition else {
            return;
        };
        self.opacity = transition.opacity_at(now);
        if now >= transition.ends_at() {
            self.opacity = transition.to;
            self.transition = None;
        }
    }

    fn transition_to(&mut self, target: f32, duration: Duration, now: Instant) {
        if (self.opacity - target).abs() <= f32::EPSILON {
            self.opacity = target;
            self.transition = None;
            self.schedule_resting(now);
            return;
        }
        if self
            .transition
            .is_some_and(|transition| (transition.to - target).abs() <= f32::EPSILON)
        {
            self.schedule_next_frame(now);
            return;
        }
        self.transition = Some(OpacityTransition {
            started_at: now,
            duration,
            from: self.opacity,
            to: target,
        });
        self.schedule_next_frame(now);
    }

    fn schedule_resting(&mut self, now: Instant) {
        if self.transition.is_some() {
            self.schedule_next_frame(now);
        } else if self.state == ScrollbarState::Resting && self.opacity > 0.0 {
            self.next_deadline = self.visible_until;
        } else {
            self.next_deadline = None;
        }
    }

    fn schedule_next_frame(&mut self, now: Instant) {
        self.next_deadline = now.checked_add(ANIMATION_FRAME_INTERVAL);
    }
}

impl Default for ScrollbarController {
    fn default() -> Self {
        Self::new(
            DEFAULT_HOLD_DURATION,
            DEFAULT_FADE_IN_DURATION,
            DEFAULT_FADE_OUT_DURATION,
        )
    }
}

#[cfg(test)]
#[path = "interaction_tests.rs"]
mod tests;
