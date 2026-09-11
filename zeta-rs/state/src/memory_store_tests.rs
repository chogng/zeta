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

fn enable(
    service: &Memories,
    scope: MemoryScope,
    command: &str,
) -> memories::MemoryPolicyMutationResult {
    service
        .update_policy(memories::UpdateMemoryPolicyRequest {
            command_id: CommandId::new(command).unwrap(),
            scope,
            expected_revision: 0,
            automatic_read: memories::MemoryReadMode::FirstInvocation,
        })
        .unwrap()
}

fn remember(service: &Memories, id: &str, scope: MemoryScope, body: &str) {
    service
        .add_user_memory(AddMemoryRequest {
            command_id: CommandId::new(format!("add-{id}")).unwrap(),
            memory_id: MemoryId::new(id).unwrap(),
            scope,
            title: format!("Memory {id}"),
            body: body.into(),
        })
        .unwrap();
}

#[test]
fn memory_read_consent_is_scoped_persistent_and_cannot_be_reenabled_by_replay() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("state.sqlite");
    let service = Memories::new(Arc::new(SqliteMemoryStore::open(&path).unwrap()));
    let profile = MemoryScope::Profile;
    let project = MemoryScope::Project {
        project_id: zeta_protocol::ProjectId::new("project-1").unwrap(),
    };
    remember(
        &service,
        "profile",
        profile.clone(),
        "Use Rust for this tool",
    );
    remember(&service, "project", project.clone(), "Rust project rules");
    let cancellation = async_utils::CancellationSource::new();
    let lookup = |service: &Memories| {
        service
            .collect_context(
                vec![profile.clone(), project.clone()],
                "How should this Rust tool work?",
                &cancellation.token(),
            )
            .unwrap()
    };
    assert!(lookup(&service).is_empty());
    assert_eq!(
        service.policy(&profile).unwrap(),
        memories::MemoryPolicy::disabled(profile.clone())
    );
    let enabled = enable(&service, project.clone(), "enable-project");
    assert_eq!(
        lookup(&service)
            .iter()
            .map(|entry| entry.citation.memory_id.as_str())
            .collect::<Vec<_>>(),
        ["project"]
    );
    let reopened = Memories::new(Arc::new(SqliteMemoryStore::open(&path).unwrap()));
    assert_eq!(lookup(&reopened), lookup(&service));
    assert_eq!(reopened.policy(&project).unwrap(), enabled.policy);
    let request = memories::UpdateMemoryPolicyRequest {
        command_id: CommandId::new("disable-project").unwrap(),
        scope: project.clone(),
        expected_revision: 1,
        automatic_read: memories::MemoryReadMode::Disabled,
    };
    let disabled = reopened.update_policy(request.clone()).unwrap();
    assert_eq!(disabled.policy.revision, 2);
    assert!(lookup(&service).is_empty());
    assert_eq!(
        reopened.update_policy(request).unwrap().disposition,
        MemoryMutationDisposition::Replayed
    );
    assert_eq!(
        enable(&service, project.clone(), "enable-project").disposition,
        MemoryMutationDisposition::Replayed
    );
    assert!(lookup(&service).is_empty());
    assert!(matches!(
        service.update_policy(memories::UpdateMemoryPolicyRequest {
            command_id: CommandId::new("stale-update").unwrap(),
            scope: project.clone(),
            expected_revision: 1,
            automatic_read: memories::MemoryReadMode::FirstInvocation,
        }),
        Err(MemoryError::RevisionConflict {
            expected: 1,
            actual: 2
        })
    ));
    assert!(matches!(
        service.update_policy(memories::UpdateMemoryPolicyRequest {
            command_id: CommandId::new("add-profile").unwrap(),
            scope: profile.clone(),
            expected_revision: 0,
            automatic_read: memories::MemoryReadMode::FirstInvocation,
        }),
        Err(MemoryError::CommandConflict)
    ));
    assert!(matches!(
        service.add_user_memory(AddMemoryRequest {
            command_id: CommandId::new("enable-project").unwrap(),
            memory_id: MemoryId::new("another").unwrap(),
            scope: profile,
            title: "title".into(),
            body: "body".into(),
        }),
        Err(MemoryError::CommandConflict)
    ));
}

#[test]
fn memory_context_and_citations_are_bounded_utf8_exact_and_removed_after_delete() {
    let root = tempfile::tempdir().unwrap();
    let service = Memories::new(Arc::new(
        SqliteMemoryStore::open(root.path().join("state.sqlite")).unwrap(),
    ));
    let scope = MemoryScope::Profile;
    let body = format!(
        "{}İstanbul Rust: keep this decision.{}",
        "背景".repeat(700),
        "资料".repeat(500)
    );
    remember(&service, "first", scope.clone(), &body);
    let search = service
        .search(SearchMemoriesRequest {
            scope: scope.clone(),
            query: "İSTANBUL".into(),
            cursor: None,
            limit: 5,
        })
        .unwrap();
    let hit = &search.matches[0];
    assert!(hit.excerpt.contains("İstanbul"));
    assert!(hit.excerpt.len() <= 1024);
    assert_eq!(
        service.read_citation(hit.citation.clone()).unwrap().body,
        hit.excerpt
    );
    let mut stale = hit.citation.clone();
    stale.revision = 2;
    assert!(matches!(
        service.read_citation(stale),
        Err(MemoryError::RevisionConflict { .. })
    ));
    let mut split = hit.citation.clone();
    split.start_byte = 1;
    assert!(matches!(
        service.read_citation(split),
        Err(MemoryError::InvalidInput(_))
    ));
    enable(&service, scope.clone(), "enable");
    for index in 0..12 {
        remember(&service, &format!("record-{index}"), scope.clone(), &body);
    }
    let cancellation = async_utils::CancellationSource::new();
    let evidence = service
        .collect_context(
            vec![scope.clone()],
            "Remember Rust decisions",
            &cancellation.token(),
        )
        .unwrap();
    assert!(!evidence.is_empty());
    assert!(evidence.len() <= 8);
    assert!(evidence.iter().all(|entry| entry.body.len() <= 4096));
    assert!(evidence.iter().map(|entry| entry.body.len()).sum::<usize>() <= 16 * 1024);
    for entry in &evidence {
        assert_eq!(
            &service.read_citation(entry.citation.clone()).unwrap(),
            entry
        );
    }
    service
        .delete(DeleteMemoryRequest {
            command_id: CommandId::new("delete-first").unwrap(),
            memory_id: hit.memory_id.clone(),
            scope: scope.clone(),
            expected_revision: 1,
        })
        .unwrap();
    assert!(matches!(
        service.read_citation(hit.citation.clone()),
        Err(MemoryError::NotFound)
    ));
    assert!(
        service
            .collect_context(vec![scope], "Rust", &cancellation.token())
            .unwrap()
            .iter()
            .all(|entry| entry.citation.memory_id != hit.memory_id)
    );
    cancellation.cancel();
    assert!(matches!(
        service.collect_context(vec![MemoryScope::Profile], "Rust", &cancellation.token()),
        Err(MemoryError::Cancelled(_))
    ));
}

#[test]
fn memory_policy_migration_preserves_existing_records_and_defaults_to_disabled() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("state.sqlite");
    {
        let service = Memories::new(Arc::new(SqliteMemoryStore::open(&path).unwrap()));
        remember(
            &service,
            "old",
            MemoryScope::Profile,
            "Existing Rust memory",
        );
    }
    let connection = rusqlite::Connection::open(&path).unwrap();
    connection
        .execute_batch(
            "DROP TABLE memory_policies; DROP TABLE memory_policy_commands;
        UPDATE zeta_schema_migrations SET version = 1 WHERE component = 'memories';",
        )
        .unwrap();
    drop(connection);
    let service = Memories::new(Arc::new(SqliteMemoryStore::open(&path).unwrap()));
    assert_eq!(
        service
            .read(ReadMemoryRequest {
                memory_id: MemoryId::new("old").unwrap(),
                scope: MemoryScope::Profile
            })
            .unwrap()
            .body,
        "Existing Rust memory"
    );
    assert_eq!(
        service
            .policy(&MemoryScope::Profile)
            .unwrap()
            .automatic_read,
        memories::MemoryReadMode::Disabled
    );
    enable(&service, MemoryScope::Profile, "enable-migrated");
}

#[test]
fn memory_search_handles_contextual_case_and_mixed_chinese_queries() {
    let root = tempfile::tempdir().unwrap();
    let service = Memories::new(Arc::new(
        SqliteMemoryStore::open(root.path().join("state.sqlite")).unwrap(),
    ));
    let body = format!("{} ΟΣ", "背景".repeat(700));
    remember(&service, "greek", MemoryScope::Profile, &body);
    let page = service
        .search(SearchMemoriesRequest {
            scope: MemoryScope::Profile,
            query: "ΟΣ".into(),
            cursor: None,
            limit: 5,
        })
        .unwrap();
    assert!(page.matches[0].excerpt.contains("ΟΣ"));
    remember(
        &service,
        "chinese",
        MemoryScope::Profile,
        "项目记忆只用于补全读取能力",
    );
    enable(&service, MemoryScope::Profile, "enable-chinese");
    let matches = service
        .collect_context(
            vec![MemoryScope::Profile],
            "继续补全memories中的记忆引用",
            &async_utils::CancellationSource::new().token(),
        )
        .unwrap();
    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0].citation.memory_id.as_str(), "chinese");
}
