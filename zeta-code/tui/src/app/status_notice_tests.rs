use super::DISPLAY_DURATION;
use super::StatusNotice;
use std::time::Duration;
use std::time::Instant;

#[test]
fn newer_notice_replaces_the_visible_notice_and_owns_its_deadline() {
    let started = Instant::now();
    let mut notice = StatusNotice::default();
    notice.show("first".into(), started);
    notice.show("second".into(), started + Duration::from_secs(1));

    assert_eq!(notice.text(), Some("second"));
    assert!(!notice.expire(started + DISPLAY_DURATION));
    assert!(notice.expire(started + Duration::from_secs(1) + DISPLAY_DURATION));
    assert_eq!(notice.text(), None);
}
