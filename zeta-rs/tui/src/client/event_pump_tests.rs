use super::{RuntimeEvent, termination_event};
use std::sync::atomic::{AtomicBool, Ordering};

#[test]
fn termination_flag_projects_one_runtime_event() {
    let requested = AtomicBool::new(false);
    assert!(termination_event(&requested).is_none());

    requested.store(true, Ordering::Release);

    assert!(matches!(
        termination_event(&requested),
        Some(RuntimeEvent::TerminationRequested)
    ));
    assert!(termination_event(&requested).is_none());
}
