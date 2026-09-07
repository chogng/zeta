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
        .write("thread", model.committed_cells(None), &mut |message| {
            output.push(message.text().into_owned());
            Ok(())
        })
        .unwrap();
    model.replace(snapshot(&text, false));
    history
        .write("thread", model.committed_cells(None), &mut |message| {
            output.push(message.text().into_owned());
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
        .write("thread", model.committed_cells(None), &mut |_| {
            panic!("live text is not committed")
        })
        .unwrap();
    model.replace(snapshot("complete", false));
    model.push_message(MessageRole::Notice, "later".into());
    assert!(
        history
            .write("thread", model.committed_cells(None), &mut |_| {
                Err(io::Error::other("write failed"))
            })
            .is_err()
    );
    let mut output = Vec::new();
    history
        .write("thread", model.committed_cells(None), &mut |message| {
            output.push(message.text().into_owned());
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
            .write(scope, model.committed_cells(None), &mut |_| {
                writes += 1;
                Ok(())
            })
            .unwrap();
    }
    assert_eq!(writes, 2);
}

#[test]
fn replacing_a_final_cell_does_not_append_its_identity_again() {
    let mut model = TranscriptModel::default();
    let mut history = TranscriptHistory::default();
    let mut output = Vec::new();
    for text in ["original", "corrected"] {
        model.replace(snapshot(text, false));
        history
            .write("thread", model.committed_cells(None), &mut |cell| {
                output.push(cell.text().into_owned());
                Ok(())
            })
            .unwrap();
    }
    assert_eq!(output, ["original"]);
    assert_eq!(model.cells()[0].history_view().text(), "corrected");
}

#[test]
fn deferred_local_command_is_committed_once_with_its_result() {
    use crate::thread::transcript::CommandStatus;
    use crate::thread::transcript::LocalCommandCompletion;
    let mut model = TranscriptModel::default();
    let mut history = TranscriptHistory::default();
    model.command_submitted("/add-dir source".into(), LocalCommandCompletion::Deferred);
    let id = model.cells()[0].cell_id().clone();
    history
        .write("thread", model.committed_cells(None), &mut |_| {
            panic!("queued command must stay live")
        })
        .unwrap();
    model.command_started("/add-dir source".into());
    history
        .write("thread", model.committed_cells(None), &mut |_| {
            panic!("running command must stay live")
        })
        .unwrap();
    model.command_completed(
        "/add-dir source".into(),
        "added".into(),
        CommandStatus::Succeeded,
    );
    let mut output = Vec::new();
    for _ in 0..2 {
        history
            .write("thread", model.committed_cells(None), &mut |cell| {
                output.push((
                    cell.text().into_owned(),
                    cell.detail().map(|text| text.into_owned()),
                ));
                Ok(())
            })
            .unwrap();
    }
    assert_eq!(model.cells()[0].cell_id(), &id);
    assert_eq!(
        output,
        [("/add-dir source".to_owned(), Some("added".to_owned()))]
    );
}

#[test]
fn failed_local_command_leaves_the_live_tail_with_its_error() {
    let mut model = TranscriptModel::default();
    model.command_submitted(
        "/theme missing".into(),
        crate::thread::transcript::LocalCommandCompletion::Deferred,
    );
    model.command_failed("/theme missing".into(), "theme not found".into());
    assert!(model.active_cells(None).is_empty());
    let cell = model.committed_cells(None)[0].history_view();
    assert_eq!(
        cell.command_status(),
        Some(crate::thread::transcript::CommandStatus::Failed)
    );
    assert_eq!(cell.detail().as_deref(), Some("theme not found"));
}

#[test]
fn panel_request_failure_keeps_its_error_without_repeating_the_command() {
    let mut model = TranscriptModel::default();
    let mut history = TranscriptHistory::default();
    let mut output = Vec::new();
    model.command_submitted(
        "/status".into(),
        crate::thread::transcript::LocalCommandCompletion::Immediate,
    );
    history
        .write("thread", model.committed_cells(None), &mut |cell| {
            output.push(cell.text().into_owned());
            Ok(())
        })
        .unwrap();
    model.command_failed("/status".into(), "status request failed".into());
    history
        .write("thread", model.committed_cells(None), &mut |cell| {
            output.push(cell.text().into_owned());
            Ok(())
        })
        .unwrap();
    assert_eq!(output, ["/status", "status request failed"]);
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
