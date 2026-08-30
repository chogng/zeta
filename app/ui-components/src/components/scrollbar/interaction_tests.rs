use std::time::Duration;
use std::time::Instant;

use super::ScrollbarController;
use super::ScrollbarPointerPresence;
use super::ScrollbarPresentation;
use super::ScrollbarState;

const HOLD: Duration = Duration::from_millis(100);
const FADE_IN: Duration = Duration::from_millis(20);
const FADE_OUT: Duration = Duration::from_millis(40);

fn controller() -> ScrollbarController {
    ScrollbarController::new(HOLD, FADE_IN, FADE_OUT)
}

#[test]
fn activity_fades_in_holds_and_fades_out() {
    let now = Instant::now();
    let mut controller = controller();

    controller.activity(now);
    assert_eq!(
        controller.presentation(),
        ScrollbarPresentation::new(ScrollbarState::Resting, 0.0)
    );
    assert!(controller.advance(now + FADE_IN));
    assert_eq!(controller.presentation().opacity(), 1.0);
    assert!(!controller.advance(now + HOLD));
    assert!(controller.advance(now + HOLD + Duration::from_millis(20)));
    assert_eq!(controller.presentation().opacity(), 0.5);
    assert!(controller.advance(now + HOLD + FADE_OUT));
    assert_eq!(controller.presentation().opacity(), 0.0);
    assert_eq!(controller.next_deadline(), None);
}

#[test]
fn hover_and_drag_keep_the_scrollbar_visible_until_pointer_exit() {
    let now = Instant::now();
    let mut controller = controller();

    controller.pointer_presence(ScrollbarPointerPresence::Over, now);
    controller.advance(now + FADE_IN);
    assert_eq!(controller.presentation().state(), ScrollbarState::Hovered);
    assert_eq!(controller.presentation().opacity(), 1.0);
    assert_eq!(controller.next_deadline(), None);

    controller.begin_drag(now + FADE_IN);
    assert_eq!(controller.presentation().state(), ScrollbarState::Active);
    controller.end_drag(
        ScrollbarPointerPresence::Outside,
        now + FADE_IN + Duration::from_millis(1),
    );
    assert_eq!(controller.presentation().state(), ScrollbarState::Resting);
    assert!(controller.next_deadline().is_some());
}

#[test]
fn cancel_immediately_hides_and_stops_animation() {
    let now = Instant::now();
    let mut controller = controller();
    controller.activity(now);

    controller.cancel();

    assert_eq!(controller.presentation().opacity(), 0.0);
    assert_eq!(controller.next_deadline(), None);
}
