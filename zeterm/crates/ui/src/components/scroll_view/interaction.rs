use std::time::{Duration, Instant};

use zui::AnimationEasing;
use zui::ScalarAnimation;

const DEFAULT_HOLD_DURATION: Duration = Duration::from_millis(700);
const DEFAULT_FADE_IN_DURATION: Duration = Duration::from_millis(120);
const DEFAULT_FADE_OUT_DURATION: Duration = Duration::from_millis(220);

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
    opacity: ScalarAnimation,
    visible_until: Option<Instant>,
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
            opacity: ScalarAnimation::new(0.0),
            visible_until: None,
        }
    }

    pub fn presentation(&self) -> ScrollbarPresentation {
        ScrollbarPresentation::new(self.state, self.opacity.value())
    }

    pub const fn next_deadline(&self) -> Option<Instant> {
        match self.opacity.next_deadline() {
            Some(deadline) => Some(deadline),
            None => self.visible_until,
        }
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
            }
        }
    }

    /// Cancels hover, capture, and animation when the owning window becomes inactive.
    pub fn cancel(&mut self) {
        self.state = ScrollbarState::Resting;
        self.opacity.snap_to(0.0);
        self.visible_until = None;
    }

    /// Advances an in-flight fade and returns whether the painted presentation changed.
    pub fn advance(&mut self, now: Instant) -> bool {
        let previous = self.presentation();
        self.resolve_transition(now);
        if self.opacity.next_deadline().is_none()
            && self.state == ScrollbarState::Resting
            && self
                .visible_until
                .is_some_and(|visible_until| now >= visible_until)
        {
            self.visible_until = None;
            self.transition_to(0.0, self.fade_out_duration, now);
            self.resolve_transition(now);
        }
        self.presentation() != previous
    }

    fn resolve_transition(&mut self, now: Instant) {
        let _ = self.opacity.advance(now);
    }

    fn transition_to(&mut self, target: f32, duration: Duration, now: Instant) {
        self.opacity
            .transition_to(target, duration, AnimationEasing::Linear, now);
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
