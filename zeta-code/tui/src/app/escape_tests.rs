use super::RootEscapeOutcome;
use super::RootEscapeSequence;
use std::time::Duration;
use std::time::Instant;

#[test]
fn second_press_inside_the_window_completes_the_sequence() {
    let mut sequence = RootEscapeSequence::default();
    let started = Instant::now();

    assert_eq!(
        sequence.press(started),
        RootEscapeOutcome::WaitingForSecondPress
    );
    assert_eq!(
        sequence.press(started + Duration::from_millis(200)),
        RootEscapeOutcome::OpenRewind
    );
}

#[test]
fn expired_or_reset_sequences_require_two_new_presses() {
    let mut sequence = RootEscapeSequence::default();
    let started = Instant::now();

    assert_eq!(
        sequence.press(started),
        RootEscapeOutcome::WaitingForSecondPress
    );
    assert_eq!(
        sequence.press(started + Duration::from_millis(600)),
        RootEscapeOutcome::WaitingForSecondPress
    );
    sequence.reset();
    assert_eq!(
        sequence.press(started + Duration::from_millis(700)),
        RootEscapeOutcome::WaitingForSecondPress
    );
    assert_eq!(
        sequence.press(started + Duration::from_millis(800)),
        RootEscapeOutcome::OpenRewind
    );
}
