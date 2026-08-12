use super::*;
use std::fs;
use zeta_code_index::CodeIndexLimits;
use zeta_code_index::CodeIndexStorage;
use zeta_policy::GrantId;
use zeta_protocol::ToolCallId;
use zeta_workspace::WorkspaceCapability;
use zeta_workspace::WorkspaceRoot;
use zeta_workspace::WorkspaceTrustDecision;
use zeta_workspace::WorkspaceTrustSource;

#[test]
fn explicit_search_code_tool_requires_its_exact_read_only_grant_and_returns_local_hits() {
    let directory = tempfile::tempdir().unwrap();
    fs::create_dir(directory.path().join(".git")).unwrap();
    fs::write(
        directory.path().join("retrieval.rs"),
        "pub fn explicit_agent_retrieval() -> bool { true }\n",
    )
    .unwrap();
    let root = WorkspaceRoot::open(directory.path()).unwrap();
    let workspace = TrustedWorkspace::require(
        root.clone(),
        WorkspaceTrustDecision::Trusted(WorkspaceTrustSource::HostConfiguration),
        WorkspaceCapability::ExecuteProcess,
    )
    .unwrap();
    let index = Arc::new(
        CodeIndex::open(root, CodeIndexStorage::Memory, CodeIndexLimits::default()).unwrap(),
    );
    index.rebuild().unwrap();
    let tool = CodeRetrievalTool::new(workspace, index, None, None);
    let call = ToolCall {
        id: ToolCallId::new("search-code-test").unwrap(),
        name: ToolName::new(CODE_RETRIEVAL_TOOL_NAME).unwrap(),
        arguments: json!({
            "query": "explicit_agent_retrieval",
            "max_results": 10
        }),
    };

    assert!(matches!(
        tool.execute(
            &call,
            &ToolAuthorization::UnsandboxedGrant {
                grant_id: GrantId::new("wrong-grant")
            },
            &zeta_async_utils::CancellationSource::new().token(),
        ),
        Err(CoreError::Policy(_))
    ));

    let ToolExecutionOutput::Success(output) = tool
        .execute(
            &call,
            &ToolAuthorization::UnsandboxedGrant {
                grant_id: GrantId::new("workspace-code-index-read-only"),
            },
            &zeta_async_utils::CancellationSource::new().token(),
        )
        .unwrap()
    else {
        panic!("search_code must return a successful result");
    };
    let output: serde_json::Value = serde_json::from_str(&output).unwrap();
    assert_eq!(output["hits"][0]["path"], "retrieval.rs");
    assert!(
        output["hits"][0]["content"]
            .as_str()
            .unwrap()
            .contains("explicit_agent_retrieval")
    );
}
