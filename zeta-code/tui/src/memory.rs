use zeta_memory_diagnostics::MemoryReport;

pub(crate) fn describe(report: &MemoryReport) -> String {
    let mut text = format!(
        "Memory diagnostics: {:?} · {} seconds · {} targets\nGrowth is evidence for investigation, not proof of a leak. UI objects count transcript cells.",
        report.status,
        report.elapsed_ms / 1000,
        report.targets.len()
    );
    for target in &report.targets {
        text.push_str(&format!(
            "\n{:?} / {:?} · PID {} · {} samples",
            target.origin, target.role, target.process_id.map(|id| id.to_string()).unwrap_or_else(|| "unavailable".into()), target.samples
        ));
        for metric in &target.latest.metrics {
            let value = metric
                .value
                .map(|value| value.to_string())
                .unwrap_or_else(|| {
                    metric
                        .unavailable
                        .map(|reason| format!("{reason:?}"))
                        .unwrap_or_else(|| "Unavailable".into())
                });
            text.push_str(&format!("\n  {:?}: {}", metric.kind, value));
        }
        for trend in &target.trends {
            text.push_str(&format!("\n  {:?}: {:?}", trend.kind, trend.finding));
        }
    }
    text.push_str("\n/memory read · /memory stop · /memory export <new-file.json>");
    text
}
