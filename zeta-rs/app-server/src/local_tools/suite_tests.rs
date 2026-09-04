use super::*;
use crate::local_tools::LocalShellToolService;
use zeta_action_policy::ActionPolicyRevision;
use zeta_file_access::Grant;
use zeta_file_access::GrantSource;
use zeta_file_access::Permission;
use zeta_file_access::Permissions;
use zeta_protocol::ToolCallId;
use zeta_protocol::ToolName;
use zeta_sandboxing::PreparedCommand;
use zeta_sandboxing::SandboxBackend;
use zeta_sandboxing::SandboxCommand;
use zeta_sandboxing::SandboxError;
use zeta_sandboxing::SandboxKind;
use zeta_sandboxing::SandboxPolicy;

struct PassThroughBackend;

impl SandboxBackend for PassThroughBackend {
    fn kind(&self) -> SandboxKind {
        SandboxKind::Unrestricted
    }

    fn prepare(
        &self,
        command: &SandboxCommand,
        _: SandboxPolicy,
        _: &Dir,
    ) -> Result<PreparedCommand, SandboxError> {
        Ok(PreparedCommand::unrestricted(command))
    }
}

#[test]
fn dir_resolution_is_bound_to_the_exact_session_and_grant() {
    let cwd_dir = tempfile::tempdir().unwrap();
    let session_dir = tempfile::tempdir().unwrap();
    let session_file = session_dir.path().join("extra.txt");
    std::fs::write(&session_file, "extra").unwrap();
    let cwd_grant = authorization(cwd_dir.path());
    let session_grant = authorization(session_dir.path());
    let dir = session_grant.dir().clone();
    let cwd_authorization = cwd_grant.authorize(Permission::ExecuteCommands).unwrap();
    let access = Arc::new(crate::dir_grants::DirGrants::default());
    let session_id = SessionId::new("session-with-extra").unwrap();
    access.add_dir(session_id.clone(), session_grant).unwrap();
    let ripgrep = RipgrepExecutable::from_path(std::env::current_exe().unwrap()).unwrap();
    let shell = LocalShellToolService::new_with_action_policy_revision(
        cwd_authorization,
        ripgrep.clone(),
        PassThroughBackend,
        ActionPolicyRevision::new("test-policy-v1"),
    )
    .unwrap();
    let agent_grep = Arc::new(AgentGrepService::new(
        zeta_config::AgentGrepBackend::Ripgrep,
        ripgrep.clone(),
        None,
    ));
    let suite = LocalToolSuite::new(shell, ripgrep, agent_grep, Arc::clone(&access));

    let resolved = suite
        .resolve(
            &session_file.display().to_string(),
            true,
            Some(&session_id),
            None,
            Permission::InspectRepository,
        )
        .unwrap();
    assert_eq!(
        resolved.absolute,
        dunce::canonicalize(&session_file).unwrap()
    );
    let read = suite
        .read_file(
            &tool_call(
                "read_file",
                serde_json::json!({
                    "path": session_file.display().to_string(),
                    "offset": null,
                    "limit": null,
                }),
            ),
            "thread",
            Some(&session_id),
            None,
        )
        .unwrap();
    assert!(matches!(read, ToolExecutionOutput::Success(text) if text.contains("extra")));
    let created = session_dir.path().join("created.txt");
    let write = suite
        .write_file(
            &tool_call(
                "write_file",
                serde_json::json!({
                    "path": created.display().to_string(),
                    "content": "created"
                }),
            ),
            "thread",
            Some(&session_id),
            None,
        )
        .unwrap();
    assert!(matches!(write, ToolExecutionOutput::Success(_)));
    assert_eq!(std::fs::read_to_string(created).unwrap(), "created");
    assert!(
        suite
            .resolve(
                &session_file.display().to_string(),
                true,
                Some(&SessionId::new("other-session").unwrap()),
                None,
                Permission::InspectRepository,
            )
            .is_err()
    );

    assert_eq!(
        access.remove_dir(&session_id, dir.canonical_path()),
        zeta_file_access::Mutation::RemovedDir
    );
    assert!(
        suite
            .resolve(
                &session_file.display().to_string(),
                true,
                Some(&session_id),
                None,
                Permission::InspectRepository,
            )
            .is_err()
    );
}

#[test]
fn work_attempt_scope_routes_every_source_root_and_excludes_session_dirs() {
    let temporary = tempfile::tempdir().unwrap();
    let source_a = temporary.path().join("source-a");
    let source_b = temporary.path().join("source-b");
    let managed_a = temporary.path().join("managed-a");
    let managed_b = temporary.path().join("managed-b");
    let output = temporary.path().join("output");
    let unrelated = temporary.path().join("unrelated");
    for path in [
        &source_a, &source_b, &managed_a, &managed_b, &output, &unrelated,
    ] {
        std::fs::create_dir(path).unwrap();
    }
    std::fs::write(managed_a.join("a.txt"), "a").unwrap();
    std::fs::write(managed_b.join("b.txt"), "b").unwrap();
    std::fs::write(output.join("test.log"), "log").unwrap();
    std::fs::write(unrelated.join("secret.txt"), "secret").unwrap();
    let source_a_dir = Dir::open_local(&source_a).unwrap();
    let source_b_dir = Dir::open_local(&source_b).unwrap();
    let cwd_authorization = authorization(&source_a)
        .authorize(Permission::ExecuteCommands)
        .unwrap();
    let access = Arc::new(crate::dir_grants::DirGrants::default());
    let session_id = SessionId::new("session").unwrap();
    let thread_id = ThreadId::new("thread").unwrap();
    access
        .add_dir(session_id.clone(), authorization(&unrelated))
        .unwrap();
    let identity = crate::dir_grants::WorkAttemptDirIdentity {
        work_run_id: zeta_protocol::WorkRunId::new("run").unwrap(),
        attempt_id: zeta_protocol::WorkAttemptId::new("attempt").unwrap(),
        execution_id: zeta_protocol::WorkExecutionId::new("execution").unwrap(),
    };
    access
        .bind_work_attempt_dirs(
            thread_id.clone(),
            identity.clone(),
            source_b_dir.id(),
            vec![
                crate::dir_grants::WorkAttemptDirRoot {
                    source: source_a_dir,
                    managed: Dir::open_local(&managed_a).unwrap(),
                },
                crate::dir_grants::WorkAttemptDirRoot {
                    source: source_b_dir,
                    managed: Dir::open_local(&managed_b).unwrap(),
                },
            ],
            Dir::open_local(&output).unwrap(),
            Dir::open_local(temporary.path()).unwrap(),
        )
        .unwrap();
    let ripgrep = RipgrepExecutable::from_path(std::env::current_exe().unwrap()).unwrap();
    let shell = LocalShellToolService::new_with_action_policy_revision(
        cwd_authorization,
        ripgrep.clone(),
        PassThroughBackend,
        ActionPolicyRevision::new("test-policy-v1"),
    )
    .unwrap();
    let suite = LocalToolSuite::new(
        shell,
        ripgrep.clone(),
        Arc::new(AgentGrepService::new(
            zeta_config::AgentGrepBackend::Ripgrep,
            ripgrep,
            None,
        )),
        Arc::clone(&access),
    );

    let primary = suite
        .resolve(
            "b.txt",
            true,
            Some(&session_id),
            Some(&thread_id),
            Permission::InspectRepository,
        )
        .unwrap();
    assert_eq!(
        primary.absolute,
        dunce::canonicalize(managed_b.join("b.txt")).unwrap()
    );
    let sandbox_scope = primary
        .thread_scope
        .as_ref()
        .unwrap()
        .sandbox_scope(&primary.authorization)
        .unwrap()
        .unwrap();
    assert_eq!(sandbox_scope.command_dir(), primary.authorization.dir());
    assert_eq!(sandbox_scope.grants().len(), 3);
    assert!(
        sandbox_scope
            .hidden_dirs()
            .iter()
            .any(|dir| dir.canonical_path() == dunce::canonicalize(temporary.path()).unwrap())
    );
    let aliased = suite
        .resolve(
            &source_a.join("a.txt").display().to_string(),
            true,
            Some(&session_id),
            Some(&thread_id),
            Permission::InspectRepository,
        )
        .unwrap();
    assert_eq!(
        aliased.absolute,
        dunce::canonicalize(managed_a.join("a.txt")).unwrap()
    );
    assert!(
        suite
            .resolve(
                &unrelated.join("secret.txt").display().to_string(),
                true,
                Some(&session_id),
                Some(&thread_id),
                Permission::InspectRepository,
            )
            .is_err()
    );
    assert!(
        suite
            .resolve(
                &output.join("test.log").display().to_string(),
                true,
                Some(&session_id),
                Some(&thread_id),
                Permission::InspectRepository,
            )
            .is_ok()
    );
    let active_authorization = access
        .thread_scope(&thread_id, Permission::InspectRepository)
        .unwrap()
        .unwrap()
        .primary()
        .clone();
    access
        .unbind_work_attempt_dirs(&thread_id, &identity)
        .unwrap();
    assert!(active_authorization.ensure_active().is_err());
    assert!(
        suite
            .resolve(
                &unrelated.join("secret.txt").display().to_string(),
                true,
                Some(&session_id),
                Some(&thread_id),
                Permission::InspectRepository,
            )
            .is_ok()
    );
}

fn authorization(path: &std::path::Path) -> Grant {
    Grant::for_environment(
        Dir::open_local(path).unwrap(),
        GrantSource::ExplicitUser,
        Permissions::new([
            Permission::ReadFiles,
            Permission::WriteFiles,
            Permission::ExecuteCommands,
            Permission::SearchFiles,
            Permission::InspectRepository,
            Permission::MutateRepository,
        ]),
    )
}

fn tool_call(name: &str, arguments: serde_json::Value) -> ToolCall {
    ToolCall {
        id: ToolCallId::new(format!("{name}-call")).unwrap(),
        name: ToolName::new(name).unwrap(),
        arguments,
    }
}
