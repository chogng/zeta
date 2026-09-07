use super::TranscriptHistory;
use crate::thread::transcript::MessageRole;
use crate::thread::transcript::TranscriptModel;
use std::io;
use zeta_app_server_protocol::protocol::transcript::ThreadTranscriptEntry;
use zeta_app_server_protocol::protocol::transcript::ThreadTranscriptSnapshot;
use zeta_protocol::ItemId;
use zeta_protocol::SessionId;
use zeta_protocol::ThreadId;
use zeta_protocol::ThreadItem;
use zeta_protocol::TurnId;

#[test]
fn completed_messages_are_written_in_full_once_despite_redraw_and_snapshot_replacement() {
    let text = (0..200)
        .map(|i| format!("第 {i} 行 🚀\n"))
        .collect::<String>();
    let mut model = TranscriptModel::default();
    model.replace(snapshot(&text, false));
    let mut history = TranscriptHistory::default();
    let mut output = Vec::new();
    history
        .write("thread", model.cells(), &mut |message| {
            output.push(message.text.clone());
            Ok(())
        })
        .unwrap();
    model.replace(snapshot(&text, false));
    history
        .write("thread", model.cells(), &mut |message| {
            output.push(message.text.clone());
            Ok(())
        })
        .unwrap();
    assert_eq!(output, vec![text]);
}

#[test]
fn streaming_blocks_later_cells_until_final_and_failed_writes_are_retried() {
    let mut model = TranscriptModel::default();
    model.replace(snapshot("partial", true));
    model.push_message(MessageRole::Notice, "later".into());
    let mut history = TranscriptHistory::default();
    history
        .write("thread", model.cells(), &mut |_| {
            panic!("live text is not committed")
        })
        .unwrap();
    model.replace(snapshot("complete", false));
    model.push_message(MessageRole::Notice, "later".into());
    assert!(
        history
            .write("thread", model.cells(), &mut |_| Err(io::Error::other(
                "write failed"
            )))
            .is_err()
    );
    let mut output = Vec::new();
    history
        .write("thread", model.cells(), &mut |message| {
            output.push(message.text.clone());
            Ok(())
        })
        .unwrap();
    assert_eq!(output, ["complete", "later"]);
}

#[test]
fn changing_thread_starts_its_own_history_even_with_identical_local_ids() {
    let mut model = TranscriptModel::default();
    model.push_message(MessageRole::User, "hello".into());
    let mut history = TranscriptHistory::default();
    let mut writes = 0;
    for scope in ["one", "two"] {
        history
            .write(scope, model.cells(), &mut |_| {
                writes += 1;
                Ok(())
            })
            .unwrap();
    }
    assert_eq!(writes, 2);
}

fn snapshot(text: &str, transient: bool) -> ThreadTranscriptSnapshot {
    let turn_id = TurnId::new("turn").unwrap();
    ThreadTranscriptSnapshot {
        session_id: SessionId::new("session").unwrap(),
        thread_id: ThreadId::new("thread").unwrap(),
        durable_sequence: 1,
        revision: 1,
        entries: vec![ThreadTranscriptEntry::Item {
            entry_id: "answer".into(),
            turn_id: turn_id.clone(),
            item: ThreadItem::AgentMessage {
                item_id: ItemId::new("answer").unwrap(),
                turn_id,
                text: text.into(),
            },
            transient,
        }],
    }
}
