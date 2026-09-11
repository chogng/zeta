use super::*;
use memories::AddMemoryRequest;
use memories::Memories;
use memories::MemoryReadMode;
use memories::UpdateMemoryPolicyRequest;
use zeta_async_utils::CancellationSource;
use zeta_extension_api::ContextSourceRequest;
use zeta_extension_api::ExtensionRegistryBuilder;
use zeta_file_access::Dir;
use zeta_file_access::Grant;
use zeta_file_access::GrantSource;
use zeta_file_access::Permissions;
use zeta_protocol::CommandId;
use zeta_protocol::ProjectId;
use zeta_protocol::SessionId;
use zeta_protocol::ThreadId;
use zeta_protocol::TurnId;

#[test]
fn automatic_memories_follow_session_projects_and_current_directory_grants() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("state.sqlite");
    let memories = Arc::new(Memories::new(Arc::new(
        zeta_state::SqliteMemoryStore::open(&path).unwrap(),
    )));
    let projects = Arc::new(ProjectCoordinator::new(Arc::new(
        zeta_state::SqliteProjectStore::open(&path).unwrap(),
    )));
    let dirs = Arc::new(DirGrants::default());
    let session = SessionId::new("session").unwrap();
    let other_session = SessionId::new("other-session").unwrap();
    let thread = ThreadId::new("thread").unwrap();
    let turn = TurnId::new("turn").unwrap();
    let project = ProjectId::new("project").unwrap();
    for (id, expected_revision, command) in [
        (
            "create",
            0,
            zeta_projects::ProjectCommand::Create {
                name: "project".into(),
                description: String::new(),
            },
        ),
        (
            "link",
            1,
            zeta_projects::ProjectCommand::LinkSession {
                session_id: session.clone(),
            },
        ),
    ] {
        projects
            .apply(zeta_projects::ProjectCommandRequest {
                command_id: CommandId::new(id).unwrap(),
                project_id: project.clone(),
                expected_revision,
                command,
            })
            .unwrap();
    }
    let work = tempfile::tempdir().unwrap();
    let dir = Dir::open_local(work.path()).unwrap();
    dirs.bind_thread_dir(thread.clone(), dir.clone());
    for (id, scope) in [
        ("profile", MemoryScope::Profile),
        (
            "project",
            MemoryScope::Project {
                project_id: project.clone(),
            },
        ),
        ("dir", MemoryScope::Dir { dir_id: dir.id() }),
    ] {
        memories
            .add_user_memory(AddMemoryRequest {
                command_id: CommandId::new(format!("add-{id}")).unwrap(),
                memory_id: memories::MemoryId::new(id).unwrap(),
                scope: scope.clone(),
                title: id.into(),
                body: format!("Rust {id} decision"),
            })
            .unwrap();
        memories
            .update_policy(UpdateMemoryPolicyRequest {
                command_id: CommandId::new(format!("enable-{id}")).unwrap(),
                scope,
                expected_revision: 0,
                automatic_read: MemoryReadMode::FirstInvocation,
            })
            .unwrap();
    }
    let mut builder = ExtensionRegistryBuilder::new();
    memories_extension::install(
        &mut builder,
        memories,
        Arc::new(MemoryScopes {
            projects: Some(projects.clone()),
            dirs: dirs.clone(),
        }),
    );
    let source = builder.build();
    let request = ContextSourceRequest {
        session_id: &session,
        thread_id: &thread,
        turn_id: &turn,
        query: "Rust",
    };
    let cancellation = CancellationSource::new();
    let read_ids = |request: &ContextSourceRequest<'_>| {
        let mut ids = source
            .collect_context(request, &cancellation.token())
            .unwrap()
            .into_iter()
            .map(|entry| {
                memories::MemoryCitation::parse(&entry.reference)
                    .unwrap()
                    .memory_id
                    .to_string()
            })
            .collect::<Vec<_>>();
        ids.sort();
        ids
    };
    assert_eq!(read_ids(&request), ["dir", "profile", "project"]);
    let other_thread = ThreadId::new("other-thread").unwrap();
    let other = ContextSourceRequest {
        session_id: &other_session,
        thread_id: &other_thread,
        turn_id: &turn,
        query: "Rust",
    };
    assert_eq!(read_ids(&other), ["profile"]);
    dirs.unbind_thread_dir(&thread);
    assert_eq!(read_ids(&request), ["profile", "project"]);
    dirs.add_dir(
        session.clone(),
        Grant::for_session_tree(
            session.clone(),
            dir.clone(),
            GrantSource::ExplicitUser,
            Permissions::new([Permission::ReadFiles]),
        ),
    )
    .unwrap();
    assert_eq!(read_ids(&request), ["dir", "profile", "project"]);
    dirs.remove_dir(&session, work.path());
    projects
        .apply(zeta_projects::ProjectCommandRequest {
            command_id: CommandId::new("unlink").unwrap(),
            project_id: project,
            expected_revision: 2,
            command: zeta_projects::ProjectCommand::UnlinkSession {
                session_id: session.clone(),
            },
        })
        .unwrap();
    assert_eq!(read_ids(&request), ["profile"]);
}
