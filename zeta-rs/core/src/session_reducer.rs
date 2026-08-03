use crate::CoreError;
use zeta_protocol::{
    ModelRef, Session, SessionCommand, SessionEvent, SessionId, SessionStatus, SessionThread,
    SessionThreadStatus, ThreadId, ThreadOrigin,
};
use zeta_session_store::{
    CURRENT_SESSION_EVENT_SCHEMA_VERSION, SessionCommandReceipt, StoredSessionEvent,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SessionSnapshot {
    pub session_id: SessionId,
    pub title: String,
    pub status: SessionStatus,
    pub model: Option<ModelRef>,
    pub sequence: u64,
    pub threads: Vec<SessionThreadSnapshot>,
    pub commands: Vec<SessionCommandSnapshot>,
}

impl SessionSnapshot {
    /// Builds the canonical public Session projection without exposing command receipts or plans.
    pub fn public_session(&self) -> Session {
        Session {
            session_id: self.session_id.clone(),
            title: self.title.clone(),
            status: self.status,
            model: self.model.clone(),
            sequence: self.sequence,
            threads: self
                .threads
                .iter()
                .map(|thread| thread.membership.clone())
                .collect(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SessionThreadSnapshot {
    pub membership: SessionThread,
    pub title: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SessionCommandSnapshot {
    pub receipt: SessionCommandReceipt,
    pub result: SessionCommandResult,
    pub response_sequence: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SessionCommandResult {
    SessionCreated,
    SessionModelChanged { model: ModelRef },
    ThreadCreated { thread_id: ThreadId },
    ThreadArchived { thread_id: ThreadId },
    SessionCompleted,
    SessionArchived,
}

/// Applies one durable Session fact to its projection without performing I/O.
pub fn reduce_session_event(
    snapshot: Option<SessionSnapshot>,
    envelope: &StoredSessionEvent,
) -> Result<SessionSnapshot, CoreError> {
    if envelope.schema_version != CURRENT_SESSION_EVENT_SCHEMA_VERSION {
        return Err(CoreError::Journal(format!(
            "unsupported Session event schema version {}",
            envelope.schema_version
        )));
    }

    let Some(mut snapshot) = snapshot else {
        if envelope.sequence != 1 {
            return Err(CoreError::Journal(
                "first Session event must have sequence 1".into(),
            ));
        }
        let SessionEvent::SessionCreated {
            session_id,
            title,
            model,
        } = &envelope.event
        else {
            return Err(CoreError::Journal(
                "first Session event must create the Session".into(),
            ));
        };
        let receipt = require_session_command(envelope)?;
        if receipt.command
            != (SessionCommand::Create {
                title: title.clone(),
                model: model.clone(),
            })
        {
            return Err(CoreError::Journal(
                "Session creation command does not match its event".into(),
            ));
        }
        return Ok(SessionSnapshot {
            session_id: session_id.clone(),
            title: title.clone(),
            status: SessionStatus::Active,
            model: model.clone(),
            sequence: 1,
            threads: Vec::new(),
            commands: vec![SessionCommandSnapshot {
                receipt: receipt.clone(),
                result: SessionCommandResult::SessionCreated,
                response_sequence: 1,
            }],
        });
    };

    if envelope.session_id != snapshot.session_id
        || envelope.event.session_id() != &snapshot.session_id
        || envelope.sequence != snapshot.sequence.saturating_add(1)
    {
        return Err(CoreError::Journal(
            "invalid Session rollout identity or sequence".into(),
        ));
    }
    if snapshot.status == SessionStatus::Archived {
        return Err(CoreError::Journal(
            "an archived Session cannot be mutated".into(),
        ));
    }

    match &envelope.event {
        SessionEvent::SessionCreated { .. } => {
            return Err(CoreError::Journal(
                "Session cannot be created more than once".into(),
            ));
        }
        SessionEvent::SessionModelChanged { model, .. } => {
            let receipt = require_new_session_command(&snapshot, envelope)?;
            if receipt.command
                != (SessionCommand::SetModel {
                    model: model.clone(),
                })
            {
                return Err(CoreError::Journal(
                    "Session model command does not match its event".into(),
                ));
            }
            snapshot.model = Some(model.clone());
            snapshot.commands.push(SessionCommandSnapshot {
                receipt: receipt.clone(),
                result: SessionCommandResult::SessionModelChanged {
                    model: model.clone(),
                },
                response_sequence: envelope.sequence,
            });
        }
        SessionEvent::ThreadCreationPlanned { thread, title, .. } => {
            if snapshot.status != SessionStatus::Active {
                return Err(CoreError::Journal(
                    "Threads can only be created in an active Session".into(),
                ));
            }
            if thread.status != SessionThreadStatus::Creating {
                return Err(CoreError::Journal(
                    "a planned Thread must start in creating status".into(),
                ));
            }
            if snapshot
                .threads
                .iter()
                .any(|existing| existing.membership.thread_id == thread.thread_id)
            {
                return Err(CoreError::Journal(format!(
                    "Thread membership already exists: {}",
                    thread.thread_id
                )));
            }
            let receipt = require_new_session_command(&snapshot, envelope)?;
            match (&receipt.command, &thread.origin) {
                (
                    SessionCommand::CreateThread {
                        title: command_title,
                    },
                    ThreadOrigin::Root,
                ) if command_title == title => {}
                (
                    SessionCommand::ForkThread {
                        parent_thread_id,
                        title: command_title,
                    },
                    ThreadOrigin::Fork {
                        parent_thread_id: origin_parent,
                        ..
                    },
                ) if command_title == title && parent_thread_id == origin_parent => {
                    let parent = snapshot
                        .threads
                        .iter()
                        .find(|candidate| candidate.membership.thread_id == *parent_thread_id)
                        .ok_or_else(|| CoreError::NotFound(parent_thread_id.to_string()))?;
                    if parent.membership.status != SessionThreadStatus::Active {
                        return Err(CoreError::Journal(
                            "a fork parent must be an active Thread".into(),
                        ));
                    }
                }
                (
                    SessionCommand::RewindThread {
                        parent_thread_id,
                        before_turn_id,
                        title: command_title,
                    },
                    ThreadOrigin::Rewind {
                        parent_thread_id: origin_parent,
                        before_turn_id: origin_turn,
                        ..
                    },
                ) if command_title == title
                    && parent_thread_id == origin_parent
                    && before_turn_id == origin_turn =>
                {
                    let parent = snapshot
                        .threads
                        .iter()
                        .find(|candidate| candidate.membership.thread_id == *parent_thread_id)
                        .ok_or_else(|| CoreError::NotFound(parent_thread_id.to_string()))?;
                    if parent.membership.status != SessionThreadStatus::Active {
                        return Err(CoreError::Journal(
                            "a rewind parent must be an active Thread".into(),
                        ));
                    }
                }
                _ => {
                    return Err(CoreError::Journal(
                        "Thread creation command does not match its plan".into(),
                    ));
                }
            }
            snapshot.threads.push(SessionThreadSnapshot {
                membership: thread.clone(),
                title: title.clone(),
            });
            snapshot.commands.push(SessionCommandSnapshot {
                receipt: receipt.clone(),
                result: SessionCommandResult::ThreadCreated {
                    thread_id: thread.thread_id.clone(),
                },
                response_sequence: envelope.sequence,
            });
        }
        SessionEvent::ThreadAttached { thread_id, .. } => {
            require_no_session_command(envelope)?;
            let thread = snapshot
                .threads
                .iter_mut()
                .find(|thread| thread.membership.thread_id == *thread_id)
                .ok_or_else(|| CoreError::NotFound(thread_id.to_string()))?;
            if thread.membership.status != SessionThreadStatus::Creating {
                return Err(CoreError::Journal(
                    "only a creating Thread can be attached".into(),
                ));
            }
            thread.membership.status = SessionThreadStatus::Active;
            update_thread_command_sequence(&mut snapshot, thread_id, envelope.sequence);
        }
        SessionEvent::ThreadArchived { thread_id, .. } => {
            let receipt = require_new_session_command(&snapshot, envelope)?;
            if receipt.command
                != (SessionCommand::ArchiveThread {
                    thread_id: thread_id.clone(),
                })
            {
                return Err(CoreError::Journal(
                    "Thread archive command does not match its event".into(),
                ));
            }
            let thread = snapshot
                .threads
                .iter_mut()
                .find(|thread| thread.membership.thread_id == *thread_id)
                .ok_or_else(|| CoreError::NotFound(thread_id.to_string()))?;
            if thread.membership.status != SessionThreadStatus::Active {
                return Err(CoreError::Journal(
                    "only an active Thread can be archived".into(),
                ));
            }
            thread.membership.status = SessionThreadStatus::Archived;
            snapshot.commands.push(SessionCommandSnapshot {
                receipt: receipt.clone(),
                result: SessionCommandResult::ThreadArchived {
                    thread_id: thread_id.clone(),
                },
                response_sequence: envelope.sequence,
            });
        }
        SessionEvent::SessionCompleted { .. } => {
            let receipt = require_new_session_command(&snapshot, envelope)?;
            if receipt.command != SessionCommand::Complete
                || snapshot.status != SessionStatus::Active
            {
                return Err(CoreError::Journal(
                    "invalid Session completion command".into(),
                ));
            }
            snapshot.status = SessionStatus::Completed;
            snapshot.commands.push(SessionCommandSnapshot {
                receipt: receipt.clone(),
                result: SessionCommandResult::SessionCompleted,
                response_sequence: envelope.sequence,
            });
        }
        SessionEvent::SessionArchived { .. } => {
            let receipt = require_new_session_command(&snapshot, envelope)?;
            if receipt.command != SessionCommand::Archive {
                return Err(CoreError::Journal("invalid Session archive command".into()));
            }
            snapshot.status = SessionStatus::Archived;
            snapshot.commands.push(SessionCommandSnapshot {
                receipt: receipt.clone(),
                result: SessionCommandResult::SessionArchived,
                response_sequence: envelope.sequence,
            });
        }
    }
    snapshot.sequence = envelope.sequence;
    Ok(snapshot)
}

fn require_session_command(
    envelope: &StoredSessionEvent,
) -> Result<&SessionCommandReceipt, CoreError> {
    envelope
        .command
        .as_ref()
        .ok_or_else(|| CoreError::Journal("Session event requires a command receipt".into()))
}

fn require_new_session_command<'a>(
    snapshot: &SessionSnapshot,
    envelope: &'a StoredSessionEvent,
) -> Result<&'a SessionCommandReceipt, CoreError> {
    let receipt = require_session_command(envelope)?;
    if snapshot
        .commands
        .iter()
        .any(|existing| existing.receipt.command_id == receipt.command_id)
    {
        return Err(CoreError::Journal(
            "Session command ID is already registered".into(),
        ));
    }
    Ok(receipt)
}

fn require_no_session_command(envelope: &StoredSessionEvent) -> Result<(), CoreError> {
    if envelope.command.is_some() {
        Err(CoreError::Journal(
            "internal Session event cannot carry a command receipt".into(),
        ))
    } else {
        Ok(())
    }
}

fn update_thread_command_sequence(
    snapshot: &mut SessionSnapshot,
    thread_id: &ThreadId,
    sequence: u64,
) {
    if let Some(command) = snapshot.commands.iter_mut().find(|command| {
        matches!(
            &command.result,
            SessionCommandResult::ThreadCreated {
                thread_id: result_thread_id
            } if result_thread_id == thread_id
        )
    }) {
        command.response_sequence = sequence;
    }
}

#[cfg(test)]
#[path = "session_reducer_tests.rs"]
mod tests;
