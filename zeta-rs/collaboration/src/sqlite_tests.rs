use crate::DocumentCollaborationOpenParams;
use crate::DocumentCollaborationReplay;
use crate::DocumentCollaborationSubmitParams;
use crate::DocumentCollaborationSubmitResult;
use crate::SqliteDocumentCollaborationRooms;
use tempfile::TempDir;

#[test]
fn sqlite_rooms_recover_ordered_snapshots_after_reopening() {
    let directory = TempDir::new().unwrap();
    let path = directory.path().join("collaboration.sqlite3");
    let room_id = {
        let rooms = SqliteDocumentCollaborationRooms::open_at(&path).unwrap();
        let opened = rooms
            .open(open_params(None, "client-a", document("initial")))
            .unwrap();
        let room_id = opened.snapshot.room_id;
        let accepted = rooms
            .submit(submit(&room_id, "client-a", 1, 0, "first"))
            .unwrap();
        assert_eq!(accepted_version(&accepted), 1);
        room_id
    };
    let rooms = SqliteDocumentCollaborationRooms::open_at(&path).unwrap();
    let joined = rooms
        .open(open_params(
            Some(room_id.clone()),
            "client-b",
            document("ignored"),
        ))
        .unwrap();
    assert_eq!(joined.snapshot.version, 1);
    assert_eq!(joined.snapshot.document, document("first"));
    let replay = rooms.replay(&room_id, 0).unwrap();
    assert!(
        matches!(replay, DocumentCollaborationReplay::Updates(updates) if updates.len() == 1 && updates[0].client_id == "client-a")
    );
}

#[test]
fn sqlite_rooms_make_submit_retries_idempotent() {
    let directory = TempDir::new().unwrap();
    let rooms =
        SqliteDocumentCollaborationRooms::open_at(directory.path().join("collaboration.sqlite3"))
            .unwrap();
    let room_id = rooms
        .open(open_params(None, "client-a", document("initial")))
        .unwrap()
        .snapshot
        .room_id;
    let first = rooms
        .submit(submit(&room_id, "client-a", 1, 0, "first"))
        .unwrap();
    let retry = rooms
        .submit(submit(&room_id, "client-a", 1, 0, "first"))
        .unwrap();
    assert_eq!(first, retry);
    assert!(
        rooms
            .submit(submit(&room_id, "client-a", 1, 0, "different"))
            .is_err()
    );
}

fn open_params(
    room_id: Option<String>,
    client_id: &str,
    document: String,
) -> DocumentCollaborationOpenParams {
    DocumentCollaborationOpenParams {
        room_id,
        client_id: client_id.into(),
        schema_id: "gama-v1".into(),
        document,
    }
}

fn submit(
    room_id: &str,
    client_id: &str,
    sequence: u64,
    base_version: u64,
    transaction: &str,
) -> DocumentCollaborationSubmitParams {
    DocumentCollaborationSubmitParams {
        room_id: room_id.into(),
        client_id: client_id.into(),
        sequence,
        base_version,
        transaction: format!(r#"{{"transaction":"{transaction}"}}"#),
        document: document(transaction),
    }
}

fn document(value: &str) -> String {
    format!(
        r#"{{"format":"zeta.document","version":1,"document":{{"type":"document","content":["{value}"]}}}}"#
    )
}

fn accepted_version(result: &DocumentCollaborationSubmitResult) -> u64 {
    match result {
        DocumentCollaborationSubmitResult::Accepted { update } => update.version,
        _ => panic!("expected an accepted collaboration update"),
    }
}
