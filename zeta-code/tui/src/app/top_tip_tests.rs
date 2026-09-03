use super::NOTICE_DURATION;
use super::REFRESH_INTERVAL;
use super::TopTip;
use std::time::Duration;
use std::time::Instant;

#[test]
fn polls_available_tips_and_wraps() {
    let started = Instant::now();
    let mut top_tip = TopTip::new(started);
    let tips = [Some("← for agents"), Some("shift+tab to cycle")];

    assert_eq!(top_tip.text(&tips), Some("← for agents"));
    assert!(!top_tip.poll(started + REFRESH_INTERVAL - Duration::from_millis(1)));
    assert!(top_tip.poll(started + REFRESH_INTERVAL));
    assert_eq!(top_tip.text(&tips), Some("shift+tab to cycle"));
    assert!(top_tip.poll(started + REFRESH_INTERVAL + REFRESH_INTERVAL));
    assert_eq!(top_tip.text(&tips), Some("← for agents"));
}

#[test]
fn notice_temporarily_replaces_the_rotating_tip() {
    let started = Instant::now();
    let mut top_tip = TopTip::new(started);
    let tips = [Some("← for agents"), Some("shift+tab to cycle")];

    top_tip.show_notice("first".into(), started);
    top_tip.show_notice(
        "Copied 246 chars to clipboard".into(),
        started + Duration::from_secs(1),
    );

    assert_eq!(top_tip.text(&tips), Some("Copied 246 chars to clipboard"));
    assert!(!top_tip.poll(started + NOTICE_DURATION));
    assert!(top_tip.poll(started + Duration::from_secs(1) + NOTICE_DURATION));
    assert_eq!(top_tip.text(&tips), Some("← for agents"));
}

#[test]
fn skips_unavailable_tips() {
    let started = Instant::now();
    let mut top_tip = TopTip::new(started);
    let tips = [None, Some("shift+tab to cycle")];

    assert_eq!(top_tip.text(&tips), Some("shift+tab to cycle"));
    assert!(top_tip.poll(started + REFRESH_INTERVAL));
    assert_eq!(top_tip.text(&tips), Some("shift+tab to cycle"));
}
