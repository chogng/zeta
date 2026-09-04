use std::collections::HashMap;
use std::collections::HashSet;
use std::collections::VecDeque;
use std::sync::Arc;
use std::sync::Condvar;
use std::sync::Mutex;

use zeta_app_server_protocol::protocol::language::LanguageCancelStatusDto;
use zeta_app_server_protocol::protocol::registry::ClientRequestSerializationScope;
use zeta_app_server_protocol::protocol::registry::SerializationAccess;
use zeta_async_utils::CancellationSource;
use zeta_async_utils::CancellationToken;

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
enum RequestSerializationKey {
    Global,
    Session(String),
    ConnectionResource {
        connection_id: u64,
        namespace: &'static str,
        resource_id: String,
    },
}

impl RequestSerializationKey {
    fn from_scope(connection_id: u64, scope: ClientRequestSerializationScope) -> (Self, Access) {
        match scope {
            ClientRequestSerializationScope::Global { access } => {
                (Self::Global, Access::from(access))
            }
            ClientRequestSerializationScope::Session { session_id, access } => {
                (Self::Session(session_id), Access::from(access))
            }
            ClientRequestSerializationScope::ConnectionResource {
                namespace,
                resource_id,
                access,
            } => (
                Self::ConnectionResource {
                    connection_id,
                    namespace,
                    resource_id,
                },
                Access::from(access),
            ),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Access {
    Exclusive,
    SharedRead,
}

impl From<SerializationAccess> for Access {
    fn from(access: SerializationAccess) -> Self {
        match access {
            SerializationAccess::Exclusive => Self::Exclusive,
            SerializationAccess::SharedRead => Self::SharedRead,
        }
    }
}

#[derive(Default)]
struct Queue {
    active: Active,
    waiting: VecDeque<Arc<Waiter>>,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
enum Active {
    #[default]
    None,
    Exclusive,
    SharedRead(usize),
}

struct Waiter {
    connection_id: u64,
    access: Access,
    state: Mutex<WaiterState>,
    ready: Condvar,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum WaiterState {
    Pending,
    Acquired,
    Cancelled,
}

impl Waiter {
    fn new(connection_id: u64, access: Access) -> Self {
        Self {
            connection_id,
            access,
            state: Mutex::new(WaiterState::Pending),
            ready: Condvar::new(),
        }
    }

    fn complete(&self, state: WaiterState) {
        *lock(&self.state) = state;
        self.ready.notify_one();
    }

    fn wait(&self, cancellation: &CancellationToken) -> WaiterState {
        let mut state = lock(&self.state);
        while *state == WaiterState::Pending {
            if cancellation.is_cancelled() {
                return WaiterState::Cancelled;
            }
            state = self
                .ready
                .wait_timeout(state, std::time::Duration::from_millis(25))
                .map(|(state, _)| state)
                .unwrap_or_else(|error| error.into_inner().0);
        }
        *state
    }
}

#[derive(Default)]
struct SchedulerState {
    queues: HashMap<RequestSerializationKey, Queue>,
    cancelled_connections: std::collections::BTreeSet<u64>,
}

#[derive(Clone, Default)]
pub(super) struct RequestScheduler {
    state: Arc<Mutex<SchedulerState>>,
}

impl RequestScheduler {
    #[cfg(test)]
    pub(super) fn acquire(
        &self,
        connection_id: u64,
        scope: ClientRequestSerializationScope,
    ) -> Result<RequestPermit, ConnectionClosed> {
        let cancellation = CancellationSource::new();
        self.acquire_with_cancellation(connection_id, scope, &cancellation.token())
    }

    pub(super) fn acquire_with_cancellation(
        &self,
        connection_id: u64,
        scope: ClientRequestSerializationScope,
        cancellation: &CancellationToken,
    ) -> Result<RequestPermit, ConnectionClosed> {
        let (key, access) = RequestSerializationKey::from_scope(connection_id, scope);
        let waiter = {
            let mut state = lock(&self.state);
            if state.cancelled_connections.contains(&connection_id) {
                return Err(ConnectionClosed);
            }
            let queue = state.queues.entry(key.clone()).or_default();
            if can_acquire_immediately(queue, access) {
                activate(queue, access);
                None
            } else {
                let waiter = Arc::new(Waiter::new(connection_id, access));
                queue.waiting.push_back(Arc::clone(&waiter));
                Some(waiter)
            }
        };

        if let Some(waiter) = waiter {
            if waiter.wait(cancellation) == WaiterState::Cancelled {
                let mut state = lock(&self.state);
                if let Some(queue) = state.queues.get_mut(&key) {
                    queue
                        .waiting
                        .retain(|candidate| !Arc::ptr_eq(candidate, &waiter));
                    if queue.active == Active::None && queue.waiting.is_empty() {
                        state.queues.remove(&key);
                    }
                }
                waiter.complete(WaiterState::Cancelled);
                return Err(ConnectionClosed);
            }
        }
        Ok(RequestPermit {
            scheduler: self.clone(),
            key: Some(key),
            access,
        })
    }

    pub(super) fn cancel_connection(&self, connection_id: u64) {
        let cancelled = {
            let mut state = lock(&self.state);
            state.cancelled_connections.insert(connection_id);
            state
                .queues
                .values_mut()
                .flat_map(|queue| {
                    let mut cancelled = Vec::new();
                    queue.waiting.retain(|waiter| {
                        if waiter.connection_id == connection_id {
                            cancelled.push(Arc::clone(waiter));
                            false
                        } else {
                            true
                        }
                    });
                    cancelled
                })
                .collect::<Vec<_>>()
        };
        for waiter in cancelled {
            waiter.complete(WaiterState::Cancelled);
        }
    }

    pub(super) fn is_connection_cancelled(&self, connection_id: u64) -> bool {
        lock(&self.state)
            .cancelled_connections
            .contains(&connection_id)
    }

    pub(super) fn finish_connection(&self, connection_id: u64) {
        lock(&self.state)
            .cancelled_connections
            .remove(&connection_id);
    }

    fn release(&self, key: RequestSerializationKey, access: Access) {
        let ready = {
            let mut state = lock(&self.state);
            let Some(queue) = state.queues.get_mut(&key) else {
                return;
            };
            deactivate(queue, access);
            let ready = if queue.active == Active::None {
                promote(queue)
            } else {
                Vec::new()
            };
            if queue.active == Active::None && queue.waiting.is_empty() {
                state.queues.remove(&key);
            }
            ready
        };
        for waiter in ready {
            waiter.complete(WaiterState::Acquired);
        }
    }

    #[cfg(test)]
    fn waiting_count(&self) -> usize {
        lock(&self.state)
            .queues
            .values()
            .map(|queue| queue.waiting.len())
            .sum()
    }
}

#[derive(Clone, Default)]
pub(super) struct RequestCancellationRegistry {
    state: Arc<Mutex<CancellationRegistryState>>,
}

#[derive(Default)]
struct CancellationRegistryState {
    active: HashMap<(u64, u64), CancellationSource>,
    request_operations: HashMap<(u64, u64), String>,
    active_operations: HashMap<(u64, String), CancellationSource>,
    requested_before_start: HashSet<(u64, String)>,
    requested_before_start_order: VecDeque<(u64, String)>,
    completed_operations: HashSet<(u64, String)>,
    completed_operation_order: VecDeque<(u64, String)>,
}

const RETAINED_OPERATION_LIMIT: usize = 1_024;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct DuplicateOperationId;

impl RequestCancellationRegistry {
    pub(super) fn start(
        &self,
        connection_id: u64,
        request_id: u64,
        operation_id: Option<String>,
    ) -> Result<CancellationToken, DuplicateOperationId> {
        let source = CancellationSource::new();
        let token = source.token();
        let mut state = lock(&self.state);
        if let Some(operation_id) = operation_id {
            let operation = (connection_id, operation_id.clone());
            if state.active_operations.contains_key(&operation)
                || state.completed_operations.contains(&operation)
            {
                return Err(DuplicateOperationId);
            }
            if state.requested_before_start.remove(&operation) {
                state
                    .requested_before_start_order
                    .retain(|queued| queued != &operation);
                source.cancel();
            }
            state.active_operations.insert(operation, source.clone());
            state
                .request_operations
                .insert((connection_id, request_id), operation_id);
        }
        state.active.insert((connection_id, request_id), source);
        Ok(token)
    }

    pub(super) fn cancel_operation(
        &self,
        connection_id: u64,
        operation_id: String,
    ) -> LanguageCancelStatusDto {
        let mut state = lock(&self.state);
        let operation = (connection_id, operation_id);
        if let Some(source) = state.active_operations.get(&operation) {
            if source.token().is_cancelled() {
                return LanguageCancelStatusDto::AlreadyRequested;
            }
            source.cancel();
            return LanguageCancelStatusDto::Requested;
        }
        if state.completed_operations.contains(&operation) {
            return LanguageCancelStatusDto::Completed;
        }
        if state.requested_before_start.contains(&operation) {
            return LanguageCancelStatusDto::AlreadyRequested;
        }
        let CancellationRegistryState {
            requested_before_start,
            requested_before_start_order,
            ..
        } = &mut *state;
        remember_bounded(
            requested_before_start,
            requested_before_start_order,
            operation,
        );
        LanguageCancelStatusDto::Requested
    }

    pub(super) fn finish(&self, connection_id: u64, request_id: u64) {
        let mut state = lock(&self.state);
        state.active.remove(&(connection_id, request_id));
        let Some(operation_id) = state
            .request_operations
            .remove(&(connection_id, request_id))
        else {
            return;
        };
        let operation = (connection_id, operation_id);
        state.active_operations.remove(&operation);
        let CancellationRegistryState {
            completed_operations,
            completed_operation_order,
            ..
        } = &mut *state;
        remember_bounded(completed_operations, completed_operation_order, operation);
    }

    pub(super) fn cancel_connection(&self, connection_id: u64) {
        let mut state = lock(&self.state);
        for ((active_connection_id, _), source) in &state.active {
            if *active_connection_id == connection_id {
                source.cancel();
            }
        }
        state
            .request_operations
            .retain(|(active_connection_id, _), _| *active_connection_id != connection_id);
        state
            .active_operations
            .retain(|(active_connection_id, _), _| *active_connection_id != connection_id);
        state
            .requested_before_start
            .retain(|(active_connection_id, _)| *active_connection_id != connection_id);
        state
            .requested_before_start_order
            .retain(|(active_connection_id, _)| *active_connection_id != connection_id);
        state
            .completed_operations
            .retain(|(active_connection_id, _)| *active_connection_id != connection_id);
        state
            .completed_operation_order
            .retain(|(active_connection_id, _)| *active_connection_id != connection_id);
    }
}

fn remember_bounded(
    retained: &mut HashSet<(u64, String)>,
    order: &mut VecDeque<(u64, String)>,
    operation: (u64, String),
) {
    if retained.insert(operation.clone()) {
        order.push_back(operation);
    }
    while retained.len() > RETAINED_OPERATION_LIMIT {
        let Some(oldest) = order.pop_front() else {
            break;
        };
        retained.remove(&oldest);
    }
}

pub(super) struct RequestPermit {
    scheduler: RequestScheduler,
    key: Option<RequestSerializationKey>,
    access: Access,
}

impl Drop for RequestPermit {
    fn drop(&mut self) {
        if let Some(key) = self.key.take() {
            self.scheduler.release(key, self.access);
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct ConnectionClosed;

fn can_acquire_immediately(queue: &Queue, access: Access) -> bool {
    match (queue.active, access) {
        (Active::None, _) => queue.waiting.is_empty(),
        (Active::SharedRead(_), Access::SharedRead) => queue.waiting.is_empty(),
        _ => false,
    }
}

fn activate(queue: &mut Queue, access: Access) {
    queue.active = match (queue.active, access) {
        (Active::None, Access::Exclusive) => Active::Exclusive,
        (Active::None, Access::SharedRead) => Active::SharedRead(1),
        (Active::SharedRead(count), Access::SharedRead) => Active::SharedRead(count + 1),
        _ => unreachable!("scheduler only activates compatible access"),
    };
}

fn deactivate(queue: &mut Queue, access: Access) {
    queue.active = match (queue.active, access) {
        (Active::Exclusive, Access::Exclusive) => Active::None,
        (Active::SharedRead(1), Access::SharedRead) => Active::None,
        (Active::SharedRead(count), Access::SharedRead) => Active::SharedRead(count - 1),
        _ => unreachable!("released access must match the active scheduler state"),
    };
}

fn promote(queue: &mut Queue) -> Vec<Arc<Waiter>> {
    let Some(first) = queue.waiting.pop_front() else {
        return Vec::new();
    };
    let access = first.access;
    let mut ready = vec![first];
    if access == Access::SharedRead {
        while queue
            .waiting
            .front()
            .is_some_and(|waiter| waiter.access == Access::SharedRead)
        {
            ready.push(queue.waiting.pop_front().expect("front waiter exists"));
        }
    }
    queue.active = match access {
        Access::Exclusive => Active::Exclusive,
        Access::SharedRead => Active::SharedRead(ready.len()),
    };
    ready
}

fn lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

#[cfg(test)]
#[path = "request_serialization_tests.rs"]
mod tests;
