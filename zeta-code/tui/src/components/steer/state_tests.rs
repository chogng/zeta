use super::Steer;

#[test]
fn pending_steers_are_removed_by_stable_identity() {
    let mut steer = Steer::default();
    let first = steer.push("first direction".into());
    let second = steer.push("second direction".into());

    assert_eq!(steer.view().items, ["first direction", "second direction"]);
    assert!(steer.remove(first));
    assert_eq!(steer.view().items, ["second direction"]);
    assert!(!steer.remove(first));
    assert!(steer.remove(second));
    assert!(steer.is_empty());
}
