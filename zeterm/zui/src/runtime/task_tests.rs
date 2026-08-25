use futures::executor::block_on;
use futures::future::AbortHandle;
use futures::future::Abortable;
use futures::future::pending;

use super::TaskEntry;
use super::TaskId;
use super::TaskRegistry;
use super::TaskScope;

#[test]
fn cancelling_a_scope_aborts_and_removes_its_tasks() {
    let (abort, registration) = AbortHandle::new_pair();
    let mut registry = TaskRegistry::default();
    registry.insert(
        TaskId(1),
        TaskEntry {
            scope: TaskScope::Application,
            abort,
        },
    );

    registry.cancel_scope(TaskScope::Application);

    assert!(registry.entries.is_empty());
    assert!(block_on(Abortable::new(pending::<()>(), registration)).is_err());
}

#[test]
fn completion_is_reported_only_for_a_still_registered_task() {
    let (abort, _) = AbortHandle::new_pair();
    let mut registry = TaskRegistry::default();
    registry.insert(
        TaskId(7),
        TaskEntry {
            scope: TaskScope::Application,
            abort,
        },
    );

    assert!(registry.complete(TaskId(7)));
    assert!(!registry.complete(TaskId(7)));
}
