use super::TranscriptState;
use super::TranscriptUpdateResult;
use zeta_protocol::ItemId;
use zeta_protocol::SessionId;
use zeta_protocol::StreamCursor;
use zeta_protocol::StreamInstanceId;
use zeta_protocol::ThreadId;
use zeta_protocol::ThreadItem;
use zeta_protocol::TurnId;
use zeta_thread_transcript::ThreadTranscriptChange;
use zeta_thread_transcript::ThreadTranscriptEntry;
use zeta_thread_transcript::ThreadTranscriptSnapshot;
use zeta_thread_transcript::ThreadTranscriptUpdateEnvelope;

#[test]
fn backend_changes_replace_entries_without_changing_their_order() {
    let mut state = TranscriptState::default();
    state.replace_snapshot(snapshot(vec![item_entry("first", "old", false)]));

    assert_eq!(
        state.apply_update(update(vec![ThreadTranscriptChange::Upsert {
            entry: item_entry("first", "new", true),
        }])),
        TranscriptUpdateResult::Applied
    );

    assert_eq!(entry_text(&state.entries()[0]), "new");
}

#[test]
fn backend_changes_append_remove_and_clear_transient_entries() {
    let mut state = TranscriptState::default();
    state.replace_snapshot(snapshot(vec![item_entry("durable", "one", false)]));
    state.apply_update(update(vec![
        ThreadTranscriptChange::Upsert {
            entry: item_entry("transient", "two", true),
        },
        ThreadTranscriptChange::Upsert {
            entry: item_entry("removed", "three", false),
        },
    ]));
    state.apply_update(update(vec![ThreadTranscriptChange::Remove {
        entry_ids: vec!["item:removed".to_owned()],
    }]));
    state.apply_update(update(vec![ThreadTranscriptChange::ClearTransient]));

    assert_eq!(state.entries(), [item_entry("durable", "one", false)]);
}

#[test]
fn updates_for_another_thread_are_ignored() {
    let mut state = TranscriptState::default();
    state.replace_snapshot(snapshot(Vec::new()));
    let mut foreign = update(vec![ThreadTranscriptChange::Upsert {
        entry: item_entry("foreign", "ignored", false),
    }]);
    foreign.thread_id = ThreadId::new("other-thread").unwrap();

    assert_eq!(state.apply_update(foreign), TranscriptUpdateResult::Ignored);
    assert!(state.entries().is_empty());
}

fn snapshot(entries: Vec<ThreadTranscriptEntry>) -> ThreadTranscriptSnapshot {
    ThreadTranscriptSnapshot {
        session_id: session_id(),
        thread_id: thread_id(),
        durable_sequence: 4,
        entries,
    }
}

fn update(changes: Vec<ThreadTranscriptChange>) -> ThreadTranscriptUpdateEnvelope {
    ThreadTranscriptUpdateEnvelope {
        session_id: session_id(),
        thread_id: thread_id(),
        durable_sequence: 4,
        stream_cursor: Some(StreamCursor {
            stream_instance_id: StreamInstanceId::new("stream").unwrap(),
            sequence: 1,
        }),
        changes,
    }
}

fn item_entry(id: &str, text: &str, transient: bool) -> ThreadTranscriptEntry {
    ThreadTranscriptEntry::Item {
        entry_id: format!("item:{id}"),
        turn_id: turn_id(),
        item: ThreadItem::AgentMessage {
            item_id: ItemId::new(id).unwrap(),
            turn_id: turn_id(),
            text: text.to_owned(),
        },
        transient,
    }
}

fn entry_text(entry: &ThreadTranscriptEntry) -> &str {
    let ThreadTranscriptEntry::Item {
        item: ThreadItem::AgentMessage { text, .. },
        ..
    } = entry
    else {
        panic!("expected Agent message entry");
    };
    text
}

fn session_id() -> SessionId {
    SessionId::new("session").unwrap()
}

fn thread_id() -> ThreadId {
    ThreadId::new("thread").unwrap()
}

fn turn_id() -> TurnId {
    TurnId::new("turn").unwrap()
}
