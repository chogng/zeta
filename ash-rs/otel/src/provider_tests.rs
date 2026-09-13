#[test]
fn production_sdk_exports_to_bounded_diagnostics() {
    let diagnostics = diagnostics::Diagnostics::default();
    let telemetry = super::Telemetry::new(diagnostics.clone());
    telemetry.record(
        diagnostics::Activity::Model,
        diagnostics::Outcome::Failed,
        std::time::Duration::from_millis(42),
    );
    telemetry.flush().unwrap();
    let snapshot = diagnostics.snapshot(Default::default());
    assert_eq!(
        snapshot.recent,
        vec![diagnostics::Observation {
            activity: diagnostics::Activity::Model,
            outcome: diagnostics::Outcome::Failed,
            elapsed_ms: 42
        }]
    );
}
