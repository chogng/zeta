use super::*;
use crate::local_tools::LocalShellToolService;
use zeta_action_policy::ActionPolicyRevision;
use zeta_protocol::ToolCallId;
use zeta_protocol::ToolName;
use zeta_sandboxing::PreparedCommand;
use zeta_sandboxing::SandboxBackend;
use zeta_sandboxing::SandboxCommand;
use zeta_sandboxing::SandboxError;
use zeta_sandboxing::SandboxKind;
use zeta_sandboxing::SandboxPolicy;
use zeta_workspace::WorkspaceAuthorization;
use zeta_workspace::WorkspaceCapability;
use zeta_workspace::WorkspaceTrustDecision;
use zeta_workspace::WorkspaceTrustSource;

struct PassThroughBackend;

impl SandboxBackend for PassThroughBackend {
    fn kind(&self) -> SandboxKind {
        SandboxKind::Unrestricted
    }

    fn prepare(
        &self,
        command: &SandboxCommand,
        _: SandboxPolicy,
        _: &WorkspaceRoot,
    ) -> Result<PreparedCommand, SandboxError> {
        Ok(PreparedCommand::unrestricted(command))
    }
}

#[test]
fn additional_root_resolution_is_bound_to_the_exact_session_and_lease() {
    let primary = tempfile::tempdir().unwrap();
    let additional = tempfile::tempdir().unwrap();
    let additional_file = additional.path().join("extra.txt");
    std::fs::write(&additional_file, "extra").unwrap();
    let primary_authorization = authorization(primary.path());
    let additional_authorization = authorization(additional.path());
    let additional_root = additional_authorization.root().clone();
    let primary_workspace = primary_authorization
        .require(WorkspaceCapability::ExecuteProcess)
        .unwrap();
    let access = Arc::new(crate::session_workspace_access::SessionWorkspaceAccess::default());
    let session_id = SessionId::new("session-with-extra").unwrap();
    access
        .add_directory(
            session_id.clone(),
            primary_authorization.root().clone(),
            additional_authorization,
            zeta_workspace_access::AdditionalDirectoryPermissions::local_file_tools(),
        )
        .unwrap();
    let ripgrep = RipgrepExecutable::from_path(std::env::current_exe().unwrap()).unwrap();
    let shell = LocalShellToolService::new_with_action_policy_revision(
        primary_workspace,
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
            &additional_file.display().to_string(),
            true,
            Some(&session_id),
            WorkspaceCapability::InspectRepository,
        )
        .unwrap();
    assert_eq!(resolved.absolute, additional_file.canonicalize().unwrap());
    let read = suite
        .read_file(
            &tool_call(
                "read_file",
                serde_json::json!({
                    "path": additional_file.display().to_string(),
                    "offset": null,
                    "limit": null,
                }),
            ),
            "thread",
            Some(&session_id),
        )
        .unwrap();
    assert!(matches!(read, ToolExecutionOutput::Success(text) if text.contains("extra")));
    let created = additional.path().join("created.txt");
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
        )
        .unwrap();
    assert!(matches!(write, ToolExecutionOutput::Success(_)));
    assert_eq!(std::fs::read_to_string(created).unwrap(), "created");
    assert!(
        suite
            .resolve(
                &additional_file.display().to_string(),
                true,
                Some(&SessionId::new("other-session").unwrap()),
                WorkspaceCapability::InspectRepository,
            )
            .is_err()
    );

    assert_eq!(
        access.remove_directory(&session_id, additional_root.canonical_path()),
        zeta_workspace_access::WorkspaceAccessMutation::RemovedDirectory
    );
    assert!(
        suite
            .resolve(
                &additional_file.display().to_string(),
                true,
                Some(&session_id),
                WorkspaceCapability::InspectRepository,
            )
            .is_err()
    );
}

fn authorization(path: &std::path::Path) -> WorkspaceAuthorization {
    WorkspaceAuthorization::new(
        WorkspaceRoot::open(path).unwrap(),
        WorkspaceTrustDecision::Trusted(WorkspaceTrustSource::ExplicitUserDecision),
    )
}

fn tool_call(name: &str, arguments: serde_json::Value) -> ToolCall {
    ToolCall {
        id: ToolCallId::new(format!("{name}-call")).unwrap(),
        name: ToolName::new(name).unwrap(),
        arguments,
    }
}
