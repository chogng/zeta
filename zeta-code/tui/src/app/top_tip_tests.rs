use super::CLIPBOARD_IMAGE_TIP;
use super::POLICY_TIP;
use super::TRANSIENT_TIP_DURATION;
use super::TopTip;
use std::time::Duration;
use std::time::Instant;

#[test]
fn stable_tip_does_not_change_with_time() {
    let started = Instant::now();
    let mut top_tip = TopTip::new();

    assert_eq!(top_tip.text(Some("← for agents")), Some("← for agents"));
    assert!(!top_tip.poll(started + Duration::from_secs(60)));
    assert_eq!(top_tip.text(Some("← for agents")), Some("← for agents"));
}

#[test]
fn policy_tip_replaces_navigation_then_disappears() {
    let started = Instant::now();
    let mut top_tip = TopTip::new();

    top_tip.show_policy_tip(started);

    assert_eq!(top_tip.text(Some("← for agents")), Some(POLICY_TIP));
    assert!(!top_tip.poll(started + TRANSIENT_TIP_DURATION - Duration::from_millis(1)));
    assert!(top_tip.poll(started + TRANSIENT_TIP_DURATION));
    assert_eq!(top_tip.text(Some("← for agents")), None);
}

#[test]
fn showing_policy_tip_again_restarts_its_lifetime() {
    let started = Instant::now();
    let shown_again = started + Duration::from_secs(4);
    let mut top_tip = TopTip::new();

    top_tip.show_policy_tip(started);
    top_tip.show_policy_tip(shown_again);

    assert!(!top_tip.poll(started + TRANSIENT_TIP_DURATION));
    assert_eq!(top_tip.text(Some("← for agents")), Some(POLICY_TIP));
    assert!(top_tip.poll(shown_again + TRANSIENT_TIP_DURATION));
    assert_eq!(top_tip.text(Some("← for agents")), None);
}

#[test]
fn existing_conversation_hides_navigation_without_showing_policy_tip() {
    let mut top_tip = TopTip::new();

    top_tip.hide_navigation();

    assert_eq!(top_tip.text(Some("← for agents")), None);
}

#[test]
fn notice_temporarily_replaces_the_stable_tip() {
    let started = Instant::now();
    let mut top_tip = TopTip::new();

    top_tip.show_notice("first".into(), started);
    top_tip.show_notice(
        "Copied 246 chars to clipboard".into(),
        started + Duration::from_secs(1),
    );

    assert_eq!(
        top_tip.text(Some("← for agents")),
        Some("Copied 246 chars to clipboard")
    );
    assert!(!top_tip.poll(started + TRANSIENT_TIP_DURATION));
    assert!(top_tip.poll(started + Duration::from_secs(1) + TRANSIENT_TIP_DURATION));
    assert_eq!(top_tip.text(Some("← for agents")), Some("← for agents"));
}

#[test]
fn notice_outlives_the_policy_tip_without_reviving_it() {
    let started = Instant::now();
    let mut top_tip = TopTip::new();

    top_tip.show_policy_tip(started);
    top_tip.show_notice("Copied".into(), started + Duration::from_secs(2));

    assert_eq!(top_tip.text(Some("← for agents")), Some("Copied"));
    assert!(top_tip.poll(started + TRANSIENT_TIP_DURATION));
    assert_eq!(top_tip.text(Some("← for agents")), Some("Copied"));
    assert!(top_tip.poll(started + Duration::from_secs(2) + TRANSIENT_TIP_DURATION));
    assert_eq!(top_tip.text(Some("← for agents")), None);
}

#[test]
fn unavailable_tip_leaves_the_row_empty() {
    let top_tip = TopTip::new();

    assert_eq!(top_tip.text(None), None);
}

#[test]
fn showing_clipboard_image_again_restarts_its_lifetime() {
    let started = Instant::now();
    let shown_again = started + Duration::from_secs(4);
    let mut top_tip = TopTip::new();

    top_tip.show_clipboard_image(started);
    top_tip.show_clipboard_image(shown_again);

    assert_eq!(
        top_tip.text(Some("← for agents")),
        Some(CLIPBOARD_IMAGE_TIP)
    );
    assert!(!top_tip.poll(started + TRANSIENT_TIP_DURATION));
    assert_eq!(
        top_tip.text(Some("← for agents")),
        Some(CLIPBOARD_IMAGE_TIP)
    );
    assert!(top_tip.poll(shown_again + TRANSIENT_TIP_DURATION));
    assert_eq!(top_tip.text(Some("← for agents")), Some("← for agents"));
}

#[test]
fn clipboard_image_tip_yields_to_notice_without_reviving_after_expiry() {
    let started = Instant::now();
    let mut top_tip = TopTip::new();

    top_tip.show_policy_tip(started);
    top_tip.show_clipboard_image(started);

    top_tip.show_notice("Copied".into(), started + Duration::from_secs(1));
    assert_eq!(top_tip.text(Some("← for agents")), Some("Copied"));

    assert!(top_tip.poll(started + TRANSIENT_TIP_DURATION));
    assert_eq!(top_tip.text(Some("← for agents")), Some("Copied"));
    assert!(top_tip.poll(started + Duration::from_secs(1) + TRANSIENT_TIP_DURATION));
    assert_eq!(top_tip.text(Some("← for agents")), None);
}

#[test]
fn hiding_clipboard_image_restores_the_underlying_tip() {
    let started = Instant::now();
    let mut top_tip = TopTip::new();

    top_tip.show_policy_tip(started);
    top_tip.show_clipboard_image(started);
    top_tip.hide_clipboard_image();
    assert_eq!(top_tip.text(Some("← for agents")), Some(POLICY_TIP));
}
