use std::time::Duration;
use std::time::Instant;

const DOUBLE_ESCAPE_WINDOW: Duration = Duration::from_millis(500);

#[derive(Debug, Default)]
pub(super) struct RootEscapeSequence {
    previous_press: Option<Instant>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum RootEscapeOutcome {
    WaitingForSecondPress,
    OpenRewind,
}

impl RootEscapeSequence {
    pub(super) fn press(&mut self, now: Instant) -> RootEscapeOutcome {
        if self
            .previous_press
            .take()
            .is_some_and(|previous| now.saturating_duration_since(previous) <= DOUBLE_ESCAPE_WINDOW)
        {
            return RootEscapeOutcome::OpenRewind;
        }
        self.previous_press = Some(now);
        RootEscapeOutcome::WaitingForSecondPress
    }

    pub(super) fn reset(&mut self) {
        self.previous_press = None;
    }
}

#[cfg(test)]
#[path = "escape_tests.rs"]
mod tests;
