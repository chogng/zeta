use crate::CoreError;
use crate::EventJournal;
use crate::IdempotencyLedger;
use crate::IdempotencyRecord;
use crate::ThreadWriterLease;
use crate::TurnStatus;
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};
use zeta_protocol::{AgentEvent, EventId, ThreadId, Timestamp, TurnId};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ThreadSnapshot {
    pub thread_id: ThreadId,
    pub title: String,
    pub sequence: u64,
    pub turns: Vec<(TurnId, TurnStatus)>,
}

pub struct ThreadManager {
    journal: Arc<dyn EventJournal>,
    writer_lease: Option<Arc<dyn ThreadWriterLease>>,
    threads: Mutex<BTreeMap<ThreadId, ThreadSnapshot>>,
    next_id: AtomicU64,
}

impl ThreadManager {
    pub fn with_journal(journal: Arc<dyn EventJournal>) -> Self {
        Self {
            journal,
            writer_lease: None,
            threads: Mutex::new(BTreeMap::new()),
            next_id: AtomicU64::new(1),
        }
    }

    /// Builds a manager that acquires the configured writer lease before each durable
    /// Thread mutation. Composition roots use this constructor with the storage adapter.
    pub fn with_journal_and_lease(
        journal: Arc<dyn EventJournal>,
        writer_lease: Arc<dyn ThreadWriterLease>,
    ) -> Self {
        Self {
            journal,
            writer_lease: Some(writer_lease),
            threads: Mutex::new(BTreeMap::new()),
            next_id: AtomicU64::new(1),
        }
    }

    pub fn start_thread(&self, title: impl Into<String>) -> Result<ThreadId, CoreError> {
        let thread_id = ThreadId::new(self.next_identifier("thread"));
        let _lease = self.acquire_writer_lease(&thread_id)?;
        let thread_title = title.into();
        let mut snapshot = ThreadSnapshot {
            thread_id: thread_id.clone(),
            title: thread_title.clone(),
            sequence: 0,
            turns: Vec::new(),
        };
        self.record(&mut snapshot, "thread.started", &thread_title)?;
        self.threads
            .lock()
            .map_err(|_| CoreError::Journal("thread state lock poisoned".into()))?
            .insert(thread_id.clone(), snapshot);
        Ok(thread_id)
    }

    pub fn start_turn(&self, thread_id: &ThreadId) -> Result<TurnId, CoreError> {
        let _lease = self.acquire_writer_lease(thread_id)?;
        let mut threads = self
            .threads
            .lock()
            .map_err(|_| CoreError::Journal("thread state lock poisoned".into()))?;
        let snapshot = threads
            .get_mut(thread_id)
            .ok_or_else(|| CoreError::NotFound(thread_id.to_string()))?;
        let turn_id = TurnId::new(self.next_identifier("turn"));
        self.record(snapshot, "turn.created", turn_id.as_str())?;
        snapshot.turns.push((turn_id.clone(), TurnStatus::Created));
        self.record(snapshot, "turn.running", turn_id.as_str())?;
        Self::set_turn_status(snapshot, &turn_id, TurnStatus::Running)?;
        Ok(turn_id)
    }

    pub fn complete_turn(&self, thread_id: &ThreadId, turn_id: &TurnId) -> Result<(), CoreError> {
        self.transition_turn(thread_id, turn_id, TurnStatus::Completed, "turn.completed")
    }

    pub fn interrupt_turn(&self, thread_id: &ThreadId, turn_id: &TurnId) -> Result<(), CoreError> {
        self.transition_turn(
            thread_id,
            turn_id,
            TurnStatus::Cancelling,
            "turn.cancelling",
        )?;
        self.transition_turn(
            thread_id,
            turn_id,
            TurnStatus::Interrupted,
            "turn.interrupted",
        )
    }

    pub fn read_thread(&self, thread_id: &ThreadId) -> Result<ThreadSnapshot, CoreError> {
        self.threads
            .lock()
            .map_err(|_| CoreError::Journal("thread state lock poisoned".into()))?
            .get(thread_id)
            .cloned()
            .ok_or_else(|| CoreError::NotFound(thread_id.to_string()))
    }

    /// Returns the in-memory projections currently loaded by this manager.
    pub fn list_threads(&self) -> Result<Vec<ThreadSnapshot>, CoreError> {
        Ok(self
            .threads
            .lock()
            .map_err(|_| CoreError::Journal("thread state lock poisoned".into()))?
            .values()
            .cloned()
            .collect())
    }

    /// Replays one Thread's durable rollout and interrupts any Turn that was in progress when the
    /// previous process stopped. The recovery markers are appended before the restored projection
    /// becomes observable.
    pub fn recover_thread(&self, events: Vec<AgentEvent>) -> Result<ThreadSnapshot, CoreError> {
        let first = events
            .first()
            .ok_or_else(|| CoreError::Journal("cannot recover an empty rollout".into()))?;
        let thread_id = first.thread_id.clone();
        let _lease = self.acquire_writer_lease(&thread_id)?;
        let mut snapshot = ThreadSnapshot {
            thread_id: thread_id.clone(),
            title: "Recovered conversation".into(),
            sequence: 0,
            turns: Vec::new(),
        };
        for event in events {
            if event.thread_id != thread_id || event.sequence != snapshot.sequence + 1 {
                return Err(CoreError::Journal("invalid thread rollout sequence".into()));
            }
            match event.kind.as_str() {
                "thread.started" => snapshot.title = event.payload,
                "turn.created" => snapshot
                    .turns
                    .push((TurnId::new(event.payload), TurnStatus::Created)),
                "turn.running" => Self::set_turn_status(
                    &mut snapshot,
                    &TurnId::new(event.payload),
                    TurnStatus::Running,
                )?,
                "turn.completed" => Self::set_turn_status(
                    &mut snapshot,
                    &TurnId::new(event.payload),
                    TurnStatus::Completed,
                )?,
                "turn.failed" => Self::set_turn_status(
                    &mut snapshot,
                    &TurnId::new(event.payload),
                    TurnStatus::Failed,
                )?,
                "turn.cancelling" => Self::set_turn_status(
                    &mut snapshot,
                    &TurnId::new(event.payload),
                    TurnStatus::Cancelling,
                )?,
                "turn.interrupted" => Self::set_turn_status(
                    &mut snapshot,
                    &TurnId::new(event.payload),
                    TurnStatus::Interrupted,
                )?,
                _ => {}
            }
            snapshot.sequence = event.sequence;
        }
        for (turn_id, status) in snapshot.turns.clone() {
            if !matches!(
                status,
                TurnStatus::Completed | TurnStatus::Failed | TurnStatus::Interrupted
            ) {
                if status != TurnStatus::Cancelling {
                    self.record(&mut snapshot, "turn.cancelling", turn_id.as_str())?;
                    Self::set_turn_status(&mut snapshot, &turn_id, TurnStatus::Cancelling)?;
                }
                self.record(&mut snapshot, "turn.interrupted", turn_id.as_str())?;
                Self::set_turn_status(&mut snapshot, &turn_id, TurnStatus::Interrupted)?;
            }
        }
        self.threads
            .lock()
            .map_err(|_| CoreError::Journal("thread state lock poisoned".into()))?
            .insert(thread_id, snapshot.clone());
        Ok(snapshot)
    }

    fn transition_turn(
        &self,
        thread_id: &ThreadId,
        turn_id: &TurnId,
        next: TurnStatus,
        event_kind: &'static str,
    ) -> Result<(), CoreError> {
        let _lease = self.acquire_writer_lease(thread_id)?;
        let mut threads = self
            .threads
            .lock()
            .map_err(|_| CoreError::Journal("thread state lock poisoned".into()))?;
        let snapshot = threads
            .get_mut(thread_id)
            .ok_or_else(|| CoreError::NotFound(thread_id.to_string()))?;
        let (_, current) = snapshot
            .turns
            .iter()
            .find(|(id, _)| id == turn_id)
            .ok_or_else(|| CoreError::NotFound(turn_id.to_string()))?;
        current.transition(next)?;
        self.record(snapshot, event_kind, turn_id.as_str())?;
        Self::set_turn_status(snapshot, turn_id, next)
    }

    fn set_turn_status(
        snapshot: &mut ThreadSnapshot,
        turn_id: &TurnId,
        next: TurnStatus,
    ) -> Result<(), CoreError> {
        let (_, current) = snapshot
            .turns
            .iter_mut()
            .find(|(id, _)| id == turn_id)
            .ok_or_else(|| CoreError::NotFound(turn_id.to_string()))?;
        *current = current.transition(next)?;
        Ok(())
    }

    fn record(
        &self,
        snapshot: &mut ThreadSnapshot,
        kind: &'static str,
        payload: &str,
    ) -> Result<(), CoreError> {
        let sequence = snapshot.sequence + 1;
        let event = AgentEvent {
            event_id: EventId(self.next_identifier("event")),
            sequence,
            thread_id: snapshot.thread_id.clone(),
            kind: kind.into(),
            payload: payload.into(),
            occurred_at: Timestamp(
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .map_err(|error| CoreError::Journal(error.to_string()))?
                    .as_millis(),
            ),
        };
        self.journal.append(&event)?;
        snapshot.sequence = sequence;
        Ok(())
    }

    fn acquire_writer_lease(
        &self,
        thread_id: &ThreadId,
    ) -> Result<Option<Box<dyn crate::LeaseGuard>>, CoreError> {
        self.writer_lease
            .as_ref()
            .map(|lease| lease.acquire(thread_id))
            .transpose()
    }

    fn next_identifier(&self, prefix: &str) -> String {
        let ordinal = self.next_id.fetch_add(1, Ordering::Relaxed);
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        format!("{prefix}_{timestamp:032x}_{ordinal:016x}")
    }
}

#[derive(Default)]
pub struct InMemoryJournal(Mutex<Vec<AgentEvent>>);

impl InMemoryJournal {
    pub fn events(&self) -> Vec<AgentEvent> {
        self.0
            .lock()
            .expect("in-memory journal lock should not be poisoned")
            .clone()
    }
}

impl EventJournal for InMemoryJournal {
    fn append(&self, event: &AgentEvent) -> Result<(), CoreError> {
        self.0
            .lock()
            .map_err(|_| CoreError::Journal("in-memory journal lock poisoned".into()))?
            .push(event.clone());
        Ok(())
    }
}

#[derive(Default)]
pub struct InMemoryIdempotencyLedger(Mutex<BTreeMap<(String, String), IdempotencyRecord>>);

impl IdempotencyLedger for InMemoryIdempotencyLedger {
    fn get(&self, method: &str, key: &str) -> Result<Option<IdempotencyRecord>, CoreError> {
        Ok(self
            .0
            .lock()
            .map_err(|_| CoreError::Journal("in-memory idempotency lock poisoned".into()))?
            .get(&(method.into(), key.into()))
            .cloned())
    }
    fn put(&self, record: IdempotencyRecord) -> Result<(), CoreError> {
        self.0
            .lock()
            .map_err(|_| CoreError::Journal("in-memory idempotency lock poisoned".into()))?
            .insert((record.method.clone(), record.key.clone()), record);
        Ok(())
    }
}
