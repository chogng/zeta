#[test]
fn retains_bounded_facts_without_losing_totals() {
    let diagnostics = super::Diagnostics::default();
    for _ in 0..300 {
        diagnostics.record(super::Observation {
            activity: super::Activity::Rpc,
            outcome: super::Outcome::Succeeded,
            elapsed_ms: 2,
        });
    }
    let snapshot = diagnostics.snapshot(Default::default());
    assert_eq!(snapshot.recent.len(), 256);
    assert_eq!(snapshot.activities[&super::Activity::Rpc].count, 300);
    assert_eq!(snapshot.activities[&super::Activity::Rpc].elapsed_ms, 600);
    let json = serde_json::to_value(snapshot).unwrap();
    assert_eq!(
        json.as_object()
            .unwrap()
            .keys()
            .cloned()
            .collect::<Vec<_>>(),
        ["activities", "build", "recent", "usage"]
    );
}
