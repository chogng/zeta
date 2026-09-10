use crate::SqliteMemoryStore;
use memories::AddMemoryRequest;
use memories::DeleteMemoryRequest;
use memories::ListMemoriesRequest;
use memories::Memories;
use memories::MemoryError;
use memories::MemoryId;
use memories::MemoryMutationDisposition;
use memories::MemoryScope;
use memories::ReadMemoryRequest;
use memories::SearchMemoriesRequest;
use std::sync::Arc;
use zeta_protocol::CommandId;

#[test]
fn sqlite_memories_persist_search_delete_and_retry_receipts() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("state.sqlite");
    let service = Memories::new(Arc::new(SqliteMemoryStore::open(&path).unwrap()));
    let add = AddMemoryRequest {
        command_id: CommandId::new("memory-add").unwrap(),
        memory_id: MemoryId::new("memory-one").unwrap(),
        scope: MemoryScope::Profile,
        title: "Preferred editor".into(),
        body: "Use Zeta for Rust work.".into(),
    };
    let committed = service.add_user_memory(add.clone()).unwrap();
    assert_eq!(committed.disposition, MemoryMutationDisposition::Committed);
    assert_eq!(
        service.add_user_memory(add.clone()).unwrap().disposition,
        MemoryMutationDisposition::Replayed
    );

    let reopened = Memories::new(Arc::new(SqliteMemoryStore::open(&path).unwrap()));
    assert_eq!(
        reopened
            .read(ReadMemoryRequest {
                memory_id: add.memory_id.clone(),
                scope: add.scope.clone(),
            })
            .unwrap()
            .body,
        add.body
    );
    assert_eq!(
        reopened
            .search(SearchMemoriesRequest {
                scope: MemoryScope::Profile,
                query: "ZETA".into(),
                cursor: None,
                limit: 10,
            })
            .unwrap()
            .matches
            .len(),
        1
    );

    let deleted = reopened
        .delete(DeleteMemoryRequest {
            command_id: CommandId::new("memory-delete").unwrap(),
            memory_id: add.memory_id.clone(),
            scope: add.scope.clone(),
            expected_revision: 1,
        })
        .unwrap();
    assert_eq!(deleted.disposition, MemoryMutationDisposition::Committed);
    assert!(matches!(
        reopened.read(ReadMemoryRequest {
            memory_id: add.memory_id.clone(),
            scope: add.scope.clone(),
        }),
        Err(MemoryError::NotFound)
    ));
    assert!(matches!(
        reopened.add_user_memory(AddMemoryRequest {
            command_id: CommandId::new("memory-add-again").unwrap(),
            ..add
        }),
        Err(MemoryError::AlreadyExists)
    ));
}

#[test]
fn pagination_cursor_rejects_catalog_changes() {
    let root = tempfile::tempdir().unwrap();
    let service = Memories::new(Arc::new(
        SqliteMemoryStore::open(root.path().join("state.sqlite")).unwrap(),
    ));
    for index in 1..=2 {
        service
            .add_user_memory(AddMemoryRequest {
                command_id: CommandId::new(format!("add-{index}")).unwrap(),
                memory_id: MemoryId::new(format!("memory-{index}")).unwrap(),
                scope: MemoryScope::Profile,
                title: format!("title {index}"),
                body: format!("body {index}"),
            })
            .unwrap();
    }
    let first = service
        .list(ListMemoriesRequest {
            scope: MemoryScope::Profile,
            cursor: None,
            limit: 1,
        })
        .unwrap();
    service
        .add_user_memory(AddMemoryRequest {
            command_id: CommandId::new("add-3").unwrap(),
            memory_id: MemoryId::new("memory-3").unwrap(),
            scope: MemoryScope::Profile,
            title: "title 3".into(),
            body: "body 3".into(),
        })
        .unwrap();
    assert!(matches!(
        service.list(ListMemoriesRequest {
            scope: MemoryScope::Profile,
            cursor: first.next_cursor,
            limit: 1,
        }),
        Err(MemoryError::StaleCursor { .. })
    ));
}
