use crate::DocumentCollaborationOpenParams;
use crate::DocumentCollaborationReplay;
use crate::DocumentCollaborationSubmitParams;
use crate::DocumentCollaborationSubmitResult;
use crate::InMemoryDocumentCollaborationRooms;

#[test]
fn in_memory_rooms_order_updates_and_replay_stale_clients() {
    let mut rooms = InMemoryDocumentCollaborationRooms::default();
    let document = r#"{"format":"zeta.document","version":1,"document":{"type":"document"}}"#;
    let opened = rooms
        .open(DocumentCollaborationOpenParams {
            room_id: None,
            client_id: "client-a".into(),
            schema_id: "gama-v1".into(),
            document: document.into(),
        })
        .unwrap();
    assert_room_id(&opened.snapshot.room_id);
    let accepted = rooms
        .submit(submit(&opened.snapshot.room_id, "client-a", 1, 0, "first"))
        .unwrap();
    assert_eq!(accepted_version(&accepted), 1);
    let replay = rooms.replay(&opened.snapshot.room_id, 0).unwrap();
    assert!(
        matches!(replay, DocumentCollaborationReplay::Updates(updates) if updates.len() == 1 && updates[0].version == 1)
    );
}

#[test]
fn in_memory_rooms_return_the_same_result_for_an_exact_retry() {
    let mut rooms = InMemoryDocumentCollaborationRooms::default();
    let room_id = open_room(&mut rooms);
    let first = rooms
        .submit(submit(&room_id, "client-a", 1, 0, "first"))
        .unwrap();
    let retried = rooms
        .submit(submit(&room_id, "client-a", 1, 0, "first"))
        .unwrap();
    assert_eq!(first, retried);
    let rejected = rooms.submit(submit(&room_id, "client-a", 1, 0, "changed"));
    assert!(rejected.is_err());
}

fn open_room(rooms: &mut InMemoryDocumentCollaborationRooms) -> String {
    rooms
        .open(DocumentCollaborationOpenParams {
            room_id: None,
            client_id: "client-a".into(),
            schema_id: "gama-v1".into(),
            document: r#"{"format":"zeta.document","version":1,"document":{"type":"document"}}"#
                .into(),
        })
        .unwrap()
        .snapshot
        .room_id
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
        document: format!(
            r#"{{"format":"zeta.document","version":1,"document":{{"type":"document","content":["{transaction}"]}}}}"#
        ),
    }
}

fn accepted_version(result: &DocumentCollaborationSubmitResult) -> u64 {
    match result {
        DocumentCollaborationSubmitResult::Accepted { update } => update.version,
        _ => panic!("expected an accepted collaboration update"),
    }
}

fn assert_room_id(room_id: &str) {
    assert_eq!(room_id.len(), "gama-".len() + 32);
    assert!(room_id.starts_with("gama-"));
    assert!(
        room_id["gama-".len()..]
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit())
    );
}
