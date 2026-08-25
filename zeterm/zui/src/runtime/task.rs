use std::collections::HashMap;
use std::future::Future;
use std::io;
use std::panic::AssertUnwindSafe;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::Weak;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;

use futures::executor::block_on;
use futures::future::AbortHandle;
use futures::future::Abortable;
use thiserror::Error;

use crate::app::AppProxy;
use crate::app::runtime_event::RuntimeEvent;
use crate::window::WindowId;

static NEXT_TASK_ID: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
struct TaskId(u64);

/// Lifetime boundary used to cancel background work with its owning application resource.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum TaskScope {
    /// Work remains active until it completes or the application exits.
    Application,
    /// Work is cancelled when the identified window closes.
    Window(WindowId),
}

struct TaskEntry {
    scope: TaskScope,
    abort: AbortHandle,
}

#[derive(Default)]
struct TaskRegistry {
    entries: HashMap<TaskId, TaskEntry>,
}

impl TaskRegistry {
    fn insert(&mut self, id: TaskId, entry: TaskEntry) {
        self.entries.insert(id, entry);
    }

    fn complete(&mut self, id: TaskId) -> bool {
        self.entries.remove(&id).is_some()
    }

    fn cancel(&mut self, id: TaskId) {
        if let Some(entry) = self.entries.remove(&id) {
            entry.abort.abort();
        }
    }

    fn cancel_scope(&mut self, scope: TaskScope) {
        let ids = self
            .entries
            .iter()
            .filter_map(|(id, entry)| (entry.scope == scope).then_some(*id))
            .collect::<Vec<_>>();
        for id in ids {
            self.cancel(id);
        }
    }

    fn cancel_all(&mut self) {
        let entries = std::mem::take(&mut self.entries);
        for entry in entries.into_values() {
            entry.abort.abort();
        }
    }
}

/// Failure to start background work on an operating-system worker thread.
#[derive(Debug, Error)]
#[error("could not start zui background task: {source}")]
pub struct TaskSpawnError {
    #[source]
    source: io::Error,
}

/// Cancellation handle for one application- or window-scoped background task.
///
/// Dropping the handle cancels the task. Call [`Task::detach`] to leave it running until its
/// owning scope ends.
#[must_use = "dropping a task cancels it; call detach to keep it running"]
pub struct Task {
    id: TaskId,
    registry: Weak<Mutex<TaskRegistry>>,
    detached: bool,
}

impl Task {
    /// Cancels the task and suppresses delivery of any eventual output event.
    pub fn cancel(mut self) {
        self.cancel_inner();
        self.detached = true;
    }

    /// Leaves the task running until it completes or its application/window scope ends.
    pub fn detach(mut self) {
        self.detached = true;
    }

    fn cancel_inner(&self) {
        if let Some(registry) = self.registry.upgrade() {
            registry.lock().expect("task registry lock").cancel(self.id);
        }
    }
}

impl Drop for Task {
    fn drop(&mut self) {
        if !self.detached {
            self.cancel_inner();
        }
    }
}

/// Cloneable executor that turns a `Send` future result into a main-thread application event.
pub struct BackgroundExecutor<T: 'static> {
    proxy: AppProxy<T>,
    registry: Arc<Mutex<TaskRegistry>>,
}

impl<T: 'static> Clone for BackgroundExecutor<T> {
    fn clone(&self) -> Self {
        Self {
            proxy: self.proxy.clone(),
            registry: self.registry.clone(),
        }
    }
}

impl<T: 'static> BackgroundExecutor<T> {
    pub(crate) fn new(proxy: AppProxy<T>) -> Self {
        Self {
            proxy,
            registry: Arc::new(Mutex::new(TaskRegistry::default())),
        }
    }

    /// Starts background work whose successful output is delivered to [`crate::app::App::user_event`].
    pub fn spawn<F>(&self, scope: TaskScope, future: F) -> Result<Task, TaskSpawnError>
    where
        T: Send,
        F: Future<Output = T> + Send + 'static,
    {
        let id = TaskId(NEXT_TASK_ID.fetch_add(1, Ordering::Relaxed));
        let (abort, registration) = AbortHandle::new_pair();
        self.registry.lock().expect("task registry lock").insert(
            id,
            TaskEntry {
                scope,
                abort: abort.clone(),
            },
        );

        let registry = self.registry.clone();
        let proxy = self.proxy.inner.clone();
        let spawn = std::thread::Builder::new()
            .name(format!("zui-task-{}", id.0))
            .spawn(move || {
                let result = std::panic::catch_unwind(AssertUnwindSafe(|| {
                    block_on(Abortable::new(future, registration))
                }));
                let deliver = registry.lock().expect("task registry lock").complete(id);
                if deliver && let Ok(Ok(event)) = result {
                    let _ = proxy.send_event(RuntimeEvent::Product(event));
                }
            });
        if let Err(source) = spawn {
            self.registry.lock().expect("task registry lock").cancel(id);
            return Err(TaskSpawnError { source });
        }
        Ok(Task {
            id,
            registry: Arc::downgrade(&self.registry),
            detached: false,
        })
    }

    pub(crate) fn cancel_window(&self, window: WindowId) {
        self.registry
            .lock()
            .expect("task registry lock")
            .cancel_scope(TaskScope::Window(window));
    }

    pub(crate) fn cancel_all(&self) {
        self.registry
            .lock()
            .expect("task registry lock")
            .cancel_all();
    }

    pub(crate) fn active_count(&self) -> usize {
        self.registry
            .lock()
            .expect("task registry lock")
            .entries
            .len()
    }
}

#[cfg(test)]
#[path = "task_tests.rs"]
mod tests;
