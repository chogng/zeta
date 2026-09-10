use super::*;
use std::sync::Arc;
use std::sync::Mutex;
use zeta_protocol::CommandId;

#[derive(Default)]
struct RecordingStore {
    add: Mutex<Option<MemoryAddCommit>>,
}

impl MemoryStore for RecordingStore {
    fn add(&self, commit: &MemoryAddCommit) -> Result<MemoryMutationResult, MemoryStoreError> {
        *self.add.lock().unwrap() = Some(commit.clone());
        Ok(MemoryMutationResult {
            disposition: MemoryMutationDisposition::Committed,
            catalog_revision: 1,
            memory: commit.memory.clone(),
        })
    }

    fn delete(&self, _: &MemoryDeleteCommit) -> Result<MemoryDeleteResult, MemoryStoreError> {
        Err(MemoryStoreError::NotFound)
    }

    fn read(&self, _: &MemoryScope, _: &MemoryId) -> Result<Memory, MemoryStoreError> {
        Err(MemoryStoreError::NotFound)
    }

    fn list(&self, _: &MemoryStoreListRequest) -> Result<MemoryStorePage, MemoryStoreError> {
        Ok(MemoryStorePage {
            catalog_revision: 0,
            memories: Vec::new(),
            has_more: false,
        })
    }

    fn search(&self, _: &MemoryStoreSearchRequest) -> Result<MemoryStorePage, MemoryStoreError> {
        Ok(MemoryStorePage {
            catalog_revision: 0,
            memories: Vec::new(),
            has_more: false,
        })
    }
}

#[test]
fn user_addition_normalizes_search_and_rejects_unbounded_content() {
    let store = Arc::new(RecordingStore::default());
    let memories = Memories::new(store.clone());
    let result = memories
        .add_user_memory(AddMemoryRequest {
            command_id: CommandId::new("add-1").unwrap(),
            memory_id: MemoryId::new("memory-1").unwrap(),
            scope: MemoryScope::Profile,
            title: "Preferred Editor".into(),
            body: "Use Zeta for Rust work.".into(),
        })
        .unwrap();
    assert_eq!(result.memory.source, MemorySource::User);
    assert_eq!(
        store
            .add
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .normalized_search_text,
        "preferred editor\nuse zeta for rust work."
    );

    assert!(
        memories
            .add_user_memory(AddMemoryRequest {
                command_id: CommandId::new("add-2").unwrap(),
                memory_id: MemoryId::new("memory-2").unwrap(),
                scope: MemoryScope::Profile,
                title: "title".into(),
                body: "x".repeat(16 * 1024 + 1),
            })
            .is_err()
    );
}

#[test]
fn identifiers_and_page_limits_are_strict() {
    assert!(MemoryId::new("").is_err());
    assert!(MemoryId::new("contains space").is_err());
    let memories = Memories::new(Arc::new(RecordingStore::default()));
    assert!(
        memories
            .list(ListMemoriesRequest {
                scope: MemoryScope::Profile,
                cursor: None,
                limit: 0,
            })
            .is_err()
    );
}
