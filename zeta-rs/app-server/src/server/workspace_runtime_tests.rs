use super::*;
use crate::local::ProviderModelService;
use crate::local_tools::LocalToolComposition;
use std::num::NonZeroU64;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use zeta_action_policy::{ActionReviewRequest, ExecutionDecision};
use zeta_async_utils::CancellationToken;
use zeta_code_index_cloud::CloudCodeIndexCapabilities;
use zeta_code_index_cloud::CloudCodeIndexDeletionSupport;
use zeta_code_index_cloud::CloudCodeIndexDestination;
use zeta_code_index_cloud::CloudCodeIndexGrant;
use zeta_code_index_cloud::CloudCodeIndexGrantId;
use zeta_code_index_cloud::CloudCodeIndexProvider;
use zeta_code_index_cloud::CloudCodeIndexProviderError;
use zeta_code_index_cloud::CloudCodeIndexProviderId;
use zeta_code_index_cloud::CloudCodeIndexProviderRegistry;
use zeta_code_index_cloud::CloudCodeIndexPublication;
use zeta_code_index_cloud::CloudCodeIndexPublicationRequest;
use zeta_code_index_cloud::CloudCodeIndexSelection;
use zeta_code_index_cloud::CloudCodeIndexState;
use zeta_config::ToolSearchConfig;
use zeta_config::ToolSearchModeConfig;
use zeta_config::{
    ConfigCommandRequest, ConfigRevision, ConfigStore, UserConfigCommand, WorkspaceTrustSetting,
};
use zeta_core::{
    ActionPolicyService, CoreError, CreateSessionRequest, CreateSessionThreadRequest,
    InMemorySessionStore, InMemoryThreadStore, NoTools, SequenceExpectation, SessionCoordinator,
    StartTurnRequest, ThreadController,
};
use zeta_model_provider::EchoModel;
use zeta_model_provider::EmbeddingInvoker;
use zeta_model_provider::EmbeddingRequest;
use zeta_model_provider::EmbeddingResponse;
use zeta_model_provider::EmbeddingVector;
use zeta_model_provider::ModelProviderError;
use zeta_model_provider_config::ModelProviderConfig;
use zeta_protocol::ModelId;
use zeta_protocol::ModelRef;
use zeta_protocol::ProviderId;
use zeta_protocol::{CommandId, UserInput};
use zeta_shell_command::RipgrepExecutable;
use zeta_workspace::WorkspaceTrustSource;

struct TrustBoundSemanticEmbedding;

impl EmbeddingInvoker for TrustBoundSemanticEmbedding {
    fn embed(&self, request: &EmbeddingRequest) -> Result<EmbeddingResponse, ModelProviderError> {
        EmbeddingResponse::new(
            request
                .inputs()
                .iter()
                .map(|_| EmbeddingVector::new(vec![1.0, 0.0]))
                .collect::<Result<Vec<_>, _>>()?,
        )
    }
}

#[test]
fn unavailable_hybrid_tool_search_remains_gated_and_reports_status() {
    let tools = WorkspaceToolPorts::new(
        ToolPort::host(Arc::new(NoTools), Arc::new(RejectPolicy)),
        None,
        None,
        None,
        &ToolSearchConfig::default(),
        &Default::default(),
        None,
    )
    .unwrap();
    let before = tools.state.lock().unwrap().registry_generation;
    let provider = ProviderId::new("ollama").unwrap();
    let model = ModelRef::new(provider.clone(), ModelId::new("nomic-embed-text").unwrap());
    let config = ToolSearchConfig {
        mode: ToolSearchModeConfig::HybridEmbedding,
        embedding_model: Some(model.clone()),
    };
    let providers =
        std::collections::BTreeMap::from([(provider.clone(), ModelProviderConfig::new(provider))]);

    tools
        .reconcile_user_config(None, &config, &providers)
        .unwrap();

    assert!(tools.state.lock().unwrap().registry_generation > before);
    assert_eq!(
        tools.tool_search_status(),
        ToolSearchEmbeddingStatus::Unavailable {
            model: Some(model),
            reason: "this App Server host does not provide semantic model invocation".into(),
        }
    );
}

#[test]
fn workspace_runtime_replaces_authority_without_replacing_connection_owned_services() {
    let first = TestWorkspace::new("first", "first.txt");
    let second = TestWorkspace::new("second", "second.txt");
    let server = server()
        .with_local_workspace_host(None, host_trust())
        .unwrap();
    let host = server.local_workspace_host.as_ref().unwrap();

    server
        .commit_trusted_workspace_runtime(first.authorization(), test_local_tools(), host)
        .unwrap();
    let trusted_tool_names = host
        .tools
        .reloadable
        .tools()
        .definitions()
        .into_iter()
        .map(|definition| definition.name.to_string())
        .collect::<std::collections::BTreeSet<_>>();
    assert!(trusted_tool_names.contains(crate::server::multi_agent_tools::SPAWN_AGENT_TOOL_NAME));
    assert!(
        trusted_tool_names.contains(crate::server::multi_agent_tools::SEND_AGENT_MESSAGE_TOOL_NAME)
    );
    assert!(trusted_tool_names.contains(crate::server::multi_agent_tools::WAIT_AGENT_TOOL_NAME));
    assert!(!trusted_tool_names.contains("browser_open"));
    host.tools.replace_host_available(true).unwrap();
    assert!(
        host.tools
            .definitions()
            .iter()
            .any(|definition| definition.name.as_str() == "browser_open")
    );
    let Ok(first_file_system) = server.file_system_service_for(None) else {
        panic!("first file system should be installed");
    };
    assert_eq!(
        first_file_system
            .read_file(Path::new("first.txt"), 1024)
            .unwrap(),
        b"first"
    );
    let Ok(first_search) = server.workspace_search_service_for(None) else {
        panic!("first search service should be installed");
    };
    let Ok(first_terminals) = server.terminal_service() else {
        panic!("first terminal service should be installed");
    };
    let Ok(first_git) = server.git_runtime_service() else {
        panic!("first Git runtime should be installed");
    };

    server
        .commit_trusted_workspace_runtime(second.authorization(), test_local_tools(), host)
        .unwrap();
    let Ok(second_file_system) = server.file_system_service_for(None) else {
        panic!("second file system should be installed");
    };
    assert_eq!(
        second_file_system
            .read_file(Path::new("second.txt"), 1024)
            .unwrap(),
        b"second"
    );
    assert!(
        second_file_system
            .read_file(Path::new("first.txt"), 1024)
            .is_err()
    );
    let Ok(second_search) = server.workspace_search_service_for(None) else {
        panic!("second search service should be installed");
    };
    let Ok(second_terminals) = server.terminal_service() else {
        panic!("second terminal service should be installed");
    };
    let Ok(second_git) = server.git_runtime_service() else {
        panic!("second Git runtime should be installed");
    };

    assert!(Arc::ptr_eq(&first_search, &second_search));
    assert!(Arc::ptr_eq(&first_terminals, &second_terminals));
    assert!(!Arc::ptr_eq(&first_git, &second_git));
}

#[test]
fn workspace_switch_rpc_requires_a_local_workspace_host() {
    let server = server();
    let mut connection = server.connection();
    let initialized = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "clientInfo": {"name": "test", "version": "1"},
            "capabilities": {}
        }
    });
    server.handle_json(&mut connection, &initialized.to_string());
    let request = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "workspace/switch",
        "params": {
            "root": std::env::current_dir().unwrap(),
            "trust": {"type": "userConfig"}
        }
    });
    let response: serde_json::Value =
        serde_json::from_str(&server.handle_json(&mut connection, &request.to_string())).unwrap();

    assert_eq!(response["error"]["message"], "WorkspaceSwitchUnavailable");
}

#[test]
fn workspace_folders_set_routes_services_by_stable_folder_id() {
    let first = TestWorkspace::new("multi-root-first", "first.txt");
    let second = TestWorkspace::new("multi-root-second", "second.txt");
    let server = server()
        .with_local_workspace_host(None, host_trust())
        .unwrap();
    let mut connection = server.connection();
    server.handle_json(
        &mut connection,
        &serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "clientInfo": {"name": "desktop", "version": "1"},
                "capabilities": {"workspaceTrustHost": {"version": 1}}
            }
        })
        .to_string(),
    );

    let response: serde_json::Value = serde_json::from_str(
        &server.handle_json(
            &mut connection,
            &serde_json::json!({
                "jsonrpc": "2.0",
                "id": 2,
                "method": "workspace/folders/set",
                "params": {"folders": [
                    {"id": "first", "root": first.path, "trust": {"type": "hostSession"}},
                    {"id": "second", "root": second.path, "trust": {"type": "hostSession"}}
                ]}
            })
            .to_string(),
        ),
    )
    .unwrap();

    assert_eq!(response["result"]["folders"][0]["id"], "first");
    assert_eq!(response["result"]["folders"][1]["id"], "second");
    let Ok(first_files) = server.file_system_service_for(Some("first")) else {
        panic!("first folder file service should be available");
    };
    let Ok(second_files) = server.file_system_service_for(Some("second")) else {
        panic!("second folder file service should be available");
    };
    assert_eq!(
        first_files.read_file(Path::new("first.txt"), 1024).unwrap(),
        b"multi-root-first"
    );
    assert_eq!(
        second_files
            .read_file(Path::new("second.txt"), 1024)
            .unwrap(),
        b"multi-root-second"
    );
    assert!(server.file_system_service_for(Some("missing")).is_err());
    let Ok(first_terminal) = server.terminal_service_for(Some("first")) else {
        panic!("first folder terminal service should be available");
    };
    let Ok(second_terminal) = server.terminal_service_for(Some("second")) else {
        panic!("second folder terminal service should be available");
    };
    assert!(!Arc::ptr_eq(&first_terminal, &second_terminal));
}

#[test]
fn workspace_switch_rpc_requires_host_capability_for_session_trust() {
    let workspace = TestWorkspace::new("rpc-session-trust", "readable.txt");
    let config = Arc::new(ConfigStore::open(workspace.path.join("trust.sqlite3")).unwrap());
    let server = server()
        .with_config_store(Arc::clone(&config))
        .with_local_workspace_host(
            None,
            WorkspaceSwitchTrustPolicy::UserConfig(Arc::clone(&config)),
        )
        .unwrap();
    let mut connection = server.connection();
    server.handle_json(
        &mut connection,
        &serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "clientInfo": {"name": "test", "version": "1"},
                "capabilities": {}
            }
        })
        .to_string(),
    );

    let response: serde_json::Value = serde_json::from_str(
        &server.handle_json(
            &mut connection,
            &serde_json::json!({
                "jsonrpc": "2.0",
                "id": 2,
                "method": "workspace/switch",
                "params": {
                    "root": workspace.path,
                    "trust": {"type": "hostSession"}
                }
            })
            .to_string(),
        ),
    )
    .unwrap();

    assert_eq!(response["error"]["message"], "WorkspaceTrustRequired");
    assert!(server.terminal_service().is_err());
}

#[test]
fn workspace_switch_rpc_persists_host_collected_user_trust() {
    let workspace = TestWorkspace::new("rpc-user-trust", "readable.txt");
    let root = workspace.root();
    let config_storage = tempfile::tempdir().unwrap();
    let config = Arc::new(ConfigStore::open(config_storage.path().join("trust.sqlite3")).unwrap());
    let server = server()
        .with_config_store(Arc::clone(&config))
        .with_local_workspace_host(
            None,
            WorkspaceSwitchTrustPolicy::UserConfig(Arc::clone(&config)),
        )
        .unwrap();
    let mut connection = server.connection();
    server.handle_json(
        &mut connection,
        &serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "clientInfo": {"name": "desktop", "version": "1"},
                "capabilities": {"workspaceTrustHost": {"version": 1}}
            }
        })
        .to_string(),
    );

    let response: serde_json::Value = serde_json::from_str(
        &server.handle_json(
            &mut connection,
            &serde_json::json!({
                "jsonrpc": "2.0",
                "id": 2,
                "method": "workspace/switch",
                "params": {
                    "root": workspace.path,
                    "trust": {
                        "type": "userDecision",
                        "commandId": "desktop-trust-workspace",
                        "expectedRevision": 0,
                        "setting": "trusted"
                    }
                }
            })
            .to_string(),
        ),
    )
    .unwrap();

    assert_eq!(response["result"]["trust"], "trusted", "{response:#}");
    assert_eq!(
        config
            .read_snapshot()
            .unwrap()
            .values
            .workspace_trust
            .decision_for(&root.trust_id()),
        WorkspaceTrustDecision::Trusted(WorkspaceTrustSource::ExplicitUserDecision)
    );
    let read: serde_json::Value = serde_json::from_str(
        &server.handle_json(
            &mut connection,
            &serde_json::json!({
                "jsonrpc": "2.0",
                "id": 3,
                "method": "workspace/trust/read",
                "params": {"root": workspace.path}
            })
            .to_string(),
        ),
    )
    .unwrap();
    assert_eq!(read["result"]["setting"], "trusted");
    assert_eq!(read["result"]["state"], "trusted");
    assert!(server.terminal_service().is_ok());
}

#[test]
fn workspace_trust_management_rpc_lists_trusted_entries_sets_and_forgets_user_decisions() {
    let workspace = TestWorkspace::new("rpc-trust-management", "readable.txt");
    let restricted_workspace =
        TestWorkspace::new("rpc-trust-management-restricted", "readable.txt");
    let root = workspace.root();
    let config_storage = tempfile::tempdir().unwrap();
    let config = Arc::new(ConfigStore::open(config_storage.path().join("trust.sqlite3")).unwrap());
    let server = server()
        .with_config_store(Arc::clone(&config))
        .with_local_workspace_host(
            None,
            WorkspaceSwitchTrustPolicy::UserConfig(Arc::clone(&config)),
        )
        .unwrap();
    let mut connection = server.connection();
    server.handle_json(
        &mut connection,
        &serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "clientInfo": {"name": "desktop", "version": "1"},
                "capabilities": {"workspaceTrustHost": {"version": 1}}
            }
        })
        .to_string(),
    );

    let empty: serde_json::Value = serde_json::from_str(
        &server.handle_json(
            &mut connection,
            &serde_json::json!({
                "jsonrpc": "2.0",
                "id": 2,
                "method": "workspace/trust/list",
                "params": {}
            })
            .to_string(),
        ),
    )
    .unwrap();
    assert_eq!(empty["result"]["revision"], 0);
    assert_eq!(empty["result"]["entries"].as_array().unwrap().len(), 0);

    let restricted: serde_json::Value = serde_json::from_str(
        &server.handle_json(
            &mut connection,
            &serde_json::json!({
                "jsonrpc": "2.0",
                "id": 3,
                "method": "workspace/trust/set",
                "params": {
                    "commandId": "desktop-trust-management-restricted",
                    "expectedRevision": 0,
                    "root": restricted_workspace.path,
                    "setting": "restricted"
                }
            })
            .to_string(),
        ),
    )
    .unwrap();
    assert_eq!(restricted["result"]["revision"], 0, "{restricted:#}");
    assert!(
        config
            .read_snapshot()
            .unwrap()
            .values
            .workspace_trust
            .explicit_setting_for(&restricted_workspace.root().trust_id())
            .is_none()
    );

    let restricted_read: serde_json::Value = serde_json::from_str(
        &server.handle_json(
            &mut connection,
            &serde_json::json!({
                "jsonrpc": "2.0",
                "id": 9,
                "method": "workspace/trust/read",
                "params": {"root": restricted_workspace.path}
            })
            .to_string(),
        ),
    )
    .unwrap();
    assert!(restricted_read["result"]["setting"].is_null());
    assert_eq!(restricted_read["result"]["state"], "restricted");

    let restricted_list: serde_json::Value = serde_json::from_str(
        &server.handle_json(
            &mut connection,
            &serde_json::json!({
                "jsonrpc": "2.0",
                "id": 4,
                "method": "workspace/trust/list",
                "params": {}
            })
            .to_string(),
        ),
    )
    .unwrap();
    assert_eq!(restricted_list["result"]["revision"], 0);
    assert_eq!(
        restricted_list["result"]["entries"]
            .as_array()
            .unwrap()
            .len(),
        0
    );

    let set: serde_json::Value = serde_json::from_str(
        &server.handle_json(
            &mut connection,
            &serde_json::json!({
                "jsonrpc": "2.0",
                "id": 5,
                "method": "workspace/trust/set",
                "params": {
                    "commandId": "desktop-trust-management-set",
                    "expectedRevision": 0,
                    "root": workspace.path,
                    "setting": "trusted"
                }
            })
            .to_string(),
        ),
    )
    .unwrap();
    assert_eq!(set["result"]["revision"], 1, "{set:#}");

    let listed: serde_json::Value = serde_json::from_str(
        &server.handle_json(
            &mut connection,
            &serde_json::json!({
                "jsonrpc": "2.0",
                "id": 6,
                "method": "workspace/trust/list",
                "params": {}
            })
            .to_string(),
        ),
    )
    .unwrap();
    assert_eq!(listed["result"]["revision"], 1);
    assert_eq!(
        listed["result"]["entries"][0]["workspace"],
        root.trust_id().to_string()
    );
    assert_eq!(
        listed["result"]["entries"][0]["root"],
        root.canonical_path().to_string_lossy().to_string()
    );
    assert!(listed["result"]["entries"][0].get("setting").is_none());

    let forgotten: serde_json::Value = serde_json::from_str(
        &server.handle_json(
            &mut connection,
            &serde_json::json!({
                "jsonrpc": "2.0",
                "id": 7,
                "method": "workspace/trust/forget",
                "params": {
                    "commandId": "desktop-trust-management-forget",
                    "expectedRevision": 1,
                    "workspace": root.trust_id()
                }
            })
            .to_string(),
        ),
    )
    .unwrap();
    assert_eq!(forgotten["result"]["revision"], 2, "{forgotten:#}");

    let final_list: serde_json::Value = serde_json::from_str(
        &server.handle_json(
            &mut connection,
            &serde_json::json!({
                "jsonrpc": "2.0",
                "id": 8,
                "method": "workspace/trust/list",
                "params": {}
            })
            .to_string(),
        ),
    )
    .unwrap();
    assert_eq!(final_list["result"]["revision"], 2);
    assert_eq!(final_list["result"]["entries"].as_array().unwrap().len(), 0);
}

#[test]
fn restricted_workspace_installs_only_non_executable_services() {
    let workspace = TestWorkspace::new("restricted", "readable.txt");
    let provider = Arc::new(TrustRevocationProvider::new());
    let provider_trait: Arc<dyn CloudCodeIndexProvider> = provider;
    let providers = CloudCodeIndexProviderRegistry::new([provider_trait]).unwrap();
    let server = server()
        .with_cloud_code_index_providers(providers)
        .with_local_workspace_host(None, WorkspaceSwitchTrustPolicy::Restricted)
        .unwrap();

    assert_eq!(
        server.switch_local_workspace_root(workspace.path.clone()),
        Ok(workspace.root().canonical_path().to_path_buf())
    );
    assert!(server.file_system_service_for(None).is_ok());
    assert!(server.code_index_service().is_ok());
    assert!(server.cloud_code_index_service().is_err());
    assert!(server.git_runtime_service().is_ok());
    assert!(server.workspace_search_service_for(None).is_err());
    assert!(server.terminal_service().is_err());
    server
        .local_workspace_host
        .as_ref()
        .unwrap()
        .tools
        .replace_host_available(true)
        .unwrap();
    assert!(
        server
            .local_workspace_host
            .as_ref()
            .unwrap()
            .tools
            .reloadable
            .tools()
            .definitions()
            .is_empty()
    );
}

#[test]
fn browser_tools_follow_capable_connection_lifecycle_in_a_trusted_workspace() {
    let workspace = TestWorkspace::new("browser-capability", "readable.txt");
    let server = server()
        .with_local_workspace_host(None, host_trust())
        .unwrap();
    assert_eq!(
        server.switch_local_workspace_root(workspace.path.clone()),
        Ok(workspace.root().canonical_path().to_path_buf())
    );
    let tools = &server.local_workspace_host.as_ref().unwrap().tools;
    assert!(
        !tools
            .definitions()
            .iter()
            .any(|definition| definition.name.as_str() == "browser_open")
    );

    let mut connection = server.connection();
    let response: serde_json::Value = serde_json::from_str(
        &server.handle_json(
            &mut connection,
            &serde_json::json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "initialize",
                "params": {
                    "clientInfo": { "name": "desktop-test", "version": "1" },
                    "capabilities": {
                        "browser": { "version": 1, "observe": true, "input": false }
                    }
                }
            })
            .to_string(),
        ),
    )
    .unwrap();
    assert_eq!(response["result"]["capabilities"]["sessions"], true);
    assert!(
        !tools
            .definitions()
            .iter()
            .any(|definition| definition.name.as_str() == "browser_open")
    );

    let mut second_connection = server.connection();
    let second_response: serde_json::Value = serde_json::from_str(
        &server.handle_json(
            &mut second_connection,
            &serde_json::json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "initialize",
                "params": {
                    "clientInfo": { "name": "desktop-test-2", "version": "1" },
                    "capabilities": {
                        "browser": { "version": 1, "observe": true, "input": true }
                    }
                }
            })
            .to_string(),
        ),
    )
    .unwrap();
    assert_eq!(second_response["result"]["capabilities"]["sessions"], true);
    assert!(
        tools
            .definitions()
            .iter()
            .any(|definition| definition.name.as_str() == "browser_open")
    );
    server.close_connection(second_connection);
    assert!(
        !tools
            .definitions()
            .iter()
            .any(|definition| definition.name.as_str() == "browser_open")
    );
    server.close_connection(connection);
    assert!(
        !tools
            .definitions()
            .iter()
            .any(|definition| definition.name.as_str() == "browser_open")
    );
}

#[test]
fn workspace_activation_binds_native_skill_source() {
    let workspace = TestWorkspace::new("workspace-skills", "readable.txt");
    let skill_root = workspace.path.join(".zeta/skills/review-workspace");
    std::fs::create_dir_all(&skill_root).unwrap();
    std::fs::write(
        skill_root.join("SKILL.md"),
        "---\nname: review-workspace\ndescription: Review this Workspace\n---\n\nReview instructions.\n",
    )
    .unwrap();
    let server = server()
        .with_skill_runtime(
            zeta_skills_extension::BuiltInSkillSource::Omitted,
            Arc::new(EmptySkillConfig),
            None,
        )
        .unwrap()
        .with_local_workspace_host(None, WorkspaceSwitchTrustPolicy::Restricted)
        .unwrap();

    server
        .switch_local_workspace_root(workspace.path.clone())
        .unwrap();

    let snapshot = server
        .skills
        .as_ref()
        .unwrap()
        .list(zeta_skills_extension::SkillCatalogReload::Cached)
        .unwrap();
    assert_eq!(snapshot.entries.len(), 1);
    assert_eq!(
        snapshot.entries[0].catalog_entry.source().kind(),
        zeta_skills::SkillSourceKind::Workspace
    );
}

#[test]
fn user_config_trust_is_resolved_for_each_client_requested_root() {
    let workspace = TestWorkspace::new("config-trust", "readable.txt");
    let root = WorkspaceRoot::open(&workspace.path).unwrap();
    let config = Arc::new(ConfigStore::open(workspace.path.join("trust.sqlite3")).unwrap());
    let server = server()
        .with_local_workspace_host(
            None,
            WorkspaceSwitchTrustPolicy::UserConfig(Arc::clone(&config)),
        )
        .unwrap();

    assert_eq!(
        server.switch_local_workspace_root(workspace.path.clone()),
        Ok(root.canonical_path().to_path_buf())
    );
    assert!(server.file_system_service_for(None).is_ok());
    assert!(server.terminal_service().is_err());

    config
        .apply(ConfigCommandRequest {
            command_id: CommandId::new("trust-config-workspace").unwrap(),
            expected_revision: ConfigRevision::INITIAL,
            command: UserConfigCommand::SetWorkspaceTrust {
                workspace: root.trust_id(),
                setting: WorkspaceTrustSetting::Trusted,
                display_root: None,
            },
        })
        .unwrap();

    assert_eq!(
        server.switch_local_workspace_root(workspace.path.clone()),
        Ok(root.canonical_path().to_path_buf())
    );
    assert!(server.git_runtime_service().is_ok());
    assert!(server.workspace_search_service_for(None).is_ok());
    assert!(server.terminal_service().is_ok());
}

#[test]
fn user_config_trust_promotion_reactivates_an_active_restricted_workspace() {
    let workspace = TestWorkspace::new("config-promotion", "readable.txt");
    let root = workspace.root();
    let config = Arc::new(ConfigStore::open(workspace.path.join("trust.sqlite3")).unwrap());
    let server = server()
        .with_config_store(Arc::clone(&config))
        .with_local_workspace_host(
            None,
            WorkspaceSwitchTrustPolicy::UserConfig(Arc::clone(&config)),
        )
        .unwrap();
    server
        .switch_local_workspace_root(workspace.path.clone())
        .unwrap();
    assert!(server.terminal_service().is_err());
    assert!(server.workspace_search_service_for(None).is_err());

    let trusted = config
        .apply(ConfigCommandRequest {
            command_id: CommandId::new("promote-config-workspace").unwrap(),
            expected_revision: ConfigRevision::INITIAL,
            command: UserConfigCommand::SetWorkspaceTrust {
                workspace: root.trust_id(),
                setting: WorkspaceTrustSetting::Trusted,
                display_root: None,
            },
        })
        .unwrap();
    assert!(server.reconcile_active_workspace_trust().is_ok());

    assert!(server.terminal_service().is_ok());
    assert!(server.workspace_search_service_for(None).is_ok());
    assert_eq!(trusted.revision.get(), 1);
}

#[test]
fn user_config_revocation_removes_executable_services_but_keeps_file_access() {
    let workspace = TestWorkspace::new("config-revocation", "readable.txt");
    let root = workspace.root();
    let config = Arc::new(ConfigStore::open(workspace.path.join("trust.sqlite3")).unwrap());
    let trusted = config
        .apply(ConfigCommandRequest {
            command_id: CommandId::new("trust-revoked-workspace").unwrap(),
            expected_revision: ConfigRevision::INITIAL,
            command: UserConfigCommand::SetWorkspaceTrust {
                workspace: root.trust_id(),
                setting: WorkspaceTrustSetting::Trusted,
                display_root: None,
            },
        })
        .unwrap();
    let server = server()
        .with_local_workspace_host(
            None,
            WorkspaceSwitchTrustPolicy::UserConfig(Arc::clone(&config)),
        )
        .unwrap();
    server
        .switch_local_workspace_root(workspace.path.clone())
        .unwrap();
    assert!(server.terminal_service().is_ok());
    let session = server
        .sessions
        .create_session(CreateSessionRequest {
            command_id: CommandId::new("create-revocation-session").unwrap(),
            title: "revocation".into(),
            model: None,
            workspace: None,
        })
        .unwrap();
    let thread = server
        .sessions
        .create_thread(CreateSessionThreadRequest {
            command_id: CommandId::new("create-revocation-thread").unwrap(),
            session_id: session.session_id,
            expected_sequence: SequenceExpectation::Exact(session.sequence),
            title: "revocation".into(),
        })
        .unwrap();
    let turn = server
        .sessions
        .threads()
        .start_turn(
            &thread.thread_id,
            StartTurnRequest {
                command_id: CommandId::new("start-revocation-turn").unwrap(),
                expected_sequence: SequenceExpectation::Exact(1),
                model: None,
                policy_revision: "test-policy-v1".into(),
                approval_mode: zeta_protocol::ApprovalMode::AskPermissions,
                tool_profile: None,
                activated_skills: Vec::new(),
                input: vec![UserInput::Text {
                    text: "must be interrupted".into(),
                }],
            },
        )
        .unwrap();

    config
        .apply(ConfigCommandRequest {
            command_id: CommandId::new("restrict-revoked-workspace").unwrap(),
            expected_revision: trusted.revision,
            command: UserConfigCommand::SetWorkspaceTrust {
                workspace: root.trust_id(),
                setting: WorkspaceTrustSetting::Restricted,
                display_root: None,
            },
        })
        .unwrap();
    server
        .workspace_runtime_control()
        .unwrap()
        .reconcile_user_trust(&config.read_snapshot().unwrap().values)
        .unwrap();

    let Ok(file_system) = server.file_system_service_for(None) else {
        panic!("restricted filesystem should remain installed after trust revocation");
    };
    assert_eq!(
        file_system
            .read_file(Path::new("readable.txt"), 1024)
            .unwrap(),
        b"config-revocation"
    );
    assert!(server.code_index_service().is_ok());
    assert!(server.git_runtime_service().is_ok());
    assert!(server.workspace_search_service_for(None).is_err());
    assert!(server.terminal_service().is_err());
    assert_eq!(
        server
            .sessions
            .threads()
            .read_thread(&thread.thread_id)
            .unwrap()
            .turns
            .iter()
            .find(|candidate| candidate.turn_id == turn.turn_id)
            .unwrap()
            .status,
        TurnStatus::Interrupted
    );
    assert!(
        server
            .local_workspace_host
            .as_ref()
            .unwrap()
            .tools
            .reloadable
            .tools()
            .definitions()
            .is_empty()
    );
}

#[test]
fn user_config_revocation_removes_local_semantic_model_access() {
    let workspace = TestWorkspace::new("semantic-revocation", "source.rs");
    let root = workspace.root();
    let config = Arc::new(ConfigStore::open(workspace.path.join("trust.sqlite3")).unwrap());
    let trusted = config
        .apply(ConfigCommandRequest {
            command_id: CommandId::new("trust-semantic-workspace").unwrap(),
            expected_revision: ConfigRevision::INITIAL,
            command: UserConfigCommand::SetWorkspaceTrust {
                workspace: root.trust_id(),
                setting: WorkspaceTrustSetting::Trusted,
                display_root: None,
            },
        })
        .unwrap();
    let models = crate::CodeIndexSemanticModels::new(
        zeta_code_index_semantic::CodeIndexEmbeddingModelId::new("trust-test-v1").unwrap(),
        Arc::new(TrustBoundSemanticEmbedding),
    );
    let server = server()
        .with_code_index_semantic_models(models)
        .with_local_workspace_host(
            None,
            WorkspaceSwitchTrustPolicy::UserConfig(Arc::clone(&config)),
        )
        .unwrap();
    server
        .switch_local_workspace_root(workspace.path.clone())
        .unwrap();
    assert!(server.code_index_semantic_service().is_some());

    config
        .apply(ConfigCommandRequest {
            command_id: CommandId::new("restrict-semantic-workspace").unwrap(),
            expected_revision: trusted.revision,
            command: UserConfigCommand::SetWorkspaceTrust {
                workspace: root.trust_id(),
                setting: WorkspaceTrustSetting::Restricted,
                display_root: None,
            },
        })
        .unwrap();
    server
        .workspace_runtime_control()
        .unwrap()
        .reconcile_user_trust(&config.read_snapshot().unwrap().values)
        .unwrap();

    assert!(server.code_index_semantic_service().is_none());
    assert!(server.code_index_service().is_ok());
}

#[test]
fn user_config_revocation_deletes_cloud_grant_and_removes_cloud_runtime() {
    let workspace = TestWorkspace::new("cloud-revocation", "source.rs");
    let root = workspace.root();
    let config = Arc::new(ConfigStore::open(workspace.path.join("trust.sqlite3")).unwrap());
    let trusted = config
        .apply(ConfigCommandRequest {
            command_id: CommandId::new("trust-cloud-workspace").unwrap(),
            expected_revision: ConfigRevision::INITIAL,
            command: UserConfigCommand::SetWorkspaceTrust {
                workspace: root.trust_id(),
                setting: WorkspaceTrustSetting::Trusted,
                display_root: None,
            },
        })
        .unwrap();
    let provider = Arc::new(TrustRevocationProvider::new());
    let provider_trait: Arc<dyn CloudCodeIndexProvider> = provider.clone();
    let providers = CloudCodeIndexProviderRegistry::new([provider_trait]).unwrap();
    let server = server()
        .with_cloud_code_index_providers(providers)
        .with_local_workspace_host(
            None,
            WorkspaceSwitchTrustPolicy::UserConfig(Arc::clone(&config)),
        )
        .unwrap();
    server
        .switch_local_workspace_root(workspace.path.clone())
        .unwrap();
    let Ok(code_index) = server.code_index_service() else {
        panic!("local code index should be installed in a trusted workspace");
    };
    code_index.rebuild().unwrap();
    let Ok(controller) = server.cloud_code_index_service() else {
        panic!("cloud code-index controller should be installed in a trusted workspace");
    };
    let grant = CloudCodeIndexGrant {
        id: CloudCodeIndexGrantId::new("trust-revocation-grant").unwrap(),
        root_id: controller.root_id().as_str().to_owned(),
        destination: CloudCodeIndexDestination::new(
            CloudCodeIndexProviderId::new("trust-revocation").unwrap(),
            "tenant-a",
            "workspace-index",
        )
        .unwrap(),
        selection: CloudCodeIndexSelection::EntireIndex,
        max_egress_bytes: NonZeroU64::new(1024 * 1024).unwrap(),
    };
    assert_eq!(
        controller.authorize(grant).unwrap().state,
        CloudCodeIndexState::Granted
    );

    config
        .apply(ConfigCommandRequest {
            command_id: CommandId::new("restrict-cloud-workspace").unwrap(),
            expected_revision: trusted.revision,
            command: UserConfigCommand::SetWorkspaceTrust {
                workspace: root.trust_id(),
                setting: WorkspaceTrustSetting::Restricted,
                display_root: None,
            },
        })
        .unwrap();
    server
        .workspace_runtime_control()
        .unwrap()
        .reconcile_user_trust(&config.read_snapshot().unwrap().values)
        .unwrap();

    assert!(server.cloud_code_index_service().is_err());
    assert_eq!(provider.deletions.load(Ordering::SeqCst), 1);
}

#[test]
fn restricted_activation_retries_a_persisted_pending_cloud_deletion() {
    let workspace = TestWorkspace::new("pending-cloud-deletion", "source.rs");
    let storage = tempfile::tempdir().unwrap();
    let provider = Arc::new(TrustRevocationProvider::new());
    let provider_trait: Arc<dyn CloudCodeIndexProvider> = provider.clone();
    let providers = CloudCodeIndexProviderRegistry::new([provider_trait]).unwrap();
    let first_server = server()
        .with_cloud_code_index_storage_root(storage.path())
        .with_cloud_code_index_providers(providers)
        .with_local_workspace_host(None, host_trust())
        .unwrap();
    first_server
        .switch_local_workspace_root(workspace.path.clone())
        .unwrap();
    let Ok(code_index) = first_server.code_index_service() else {
        panic!("local code index should be installed");
    };
    code_index.rebuild().unwrap();
    let Ok(controller) = first_server.cloud_code_index_service() else {
        panic!("cloud controller should be installed");
    };
    controller.authorize(cloud_grant(&controller)).unwrap();
    provider.fail_deletions.store(true, Ordering::SeqCst);
    assert!(controller.revoke().is_err());
    drop(controller);
    drop(first_server);

    provider.fail_deletions.store(false, Ordering::SeqCst);
    let provider_trait: Arc<dyn CloudCodeIndexProvider> = provider.clone();
    let providers = CloudCodeIndexProviderRegistry::new([provider_trait]).unwrap();
    let restricted_server = server()
        .with_cloud_code_index_storage_root(storage.path())
        .with_cloud_code_index_providers(providers)
        .with_local_workspace_host(None, WorkspaceSwitchTrustPolicy::Restricted)
        .unwrap();
    restricted_server
        .switch_local_workspace_root(workspace.path.clone())
        .unwrap();

    assert!(restricted_server.cloud_code_index_service().is_err());
    assert_eq!(provider.deletions.load(Ordering::SeqCst), 2);
}

#[test]
fn active_turn_blocks_workspace_switch_without_changing_authority() {
    let first = TestWorkspace::new("busy-first", "first.txt");
    let second = TestWorkspace::new("busy-second", "second.txt");
    let server = server()
        .with_local_workspace_host(None, host_trust())
        .unwrap();
    let host = server.local_workspace_host.as_ref().unwrap();
    server
        .commit_trusted_workspace_runtime(first.authorization(), test_local_tools(), host)
        .unwrap();
    let session = server
        .sessions
        .create_session(CreateSessionRequest {
            command_id: CommandId::new("create-session").unwrap(),
            title: "session".into(),
            model: None,
            workspace: None,
        })
        .unwrap();
    let thread = server
        .sessions
        .create_thread(CreateSessionThreadRequest {
            command_id: CommandId::new("create-thread").unwrap(),
            session_id: session.session_id,
            expected_sequence: SequenceExpectation::Exact(session.sequence),
            title: "thread".into(),
        })
        .unwrap();
    server
        .sessions
        .threads()
        .start_turn(
            &thread.thread_id,
            StartTurnRequest {
                command_id: CommandId::new("start-turn").unwrap(),
                expected_sequence: SequenceExpectation::Exact(1),
                model: None,
                policy_revision: "test-policy-v1".into(),
                approval_mode: zeta_protocol::ApprovalMode::AskPermissions,
                tool_profile: None,
                activated_skills: Vec::new(),
                input: vec![UserInput::Text {
                    text: "stay in the first Workspace".into(),
                }],
            },
        )
        .unwrap();

    assert_eq!(
        server.switch_local_workspace_root(second.path.clone()),
        Err(WorkspaceRuntimeError::Busy)
    );
    let Ok(file_system) = server.file_system_service_for(None) else {
        panic!("first Workspace file system should remain installed");
    };
    assert_eq!(
        file_system.read_file(Path::new("first.txt"), 1024).unwrap(),
        b"busy-first"
    );
    assert!(
        file_system
            .read_file(Path::new("second.txt"), 1024)
            .is_err()
    );
}

fn server() -> AppServer {
    let threads = Arc::new(ThreadController::with_store(Arc::new(
        InMemoryThreadStore::default(),
    )));
    let sessions = Arc::new(SessionCoordinator::with_store(
        Arc::new(InMemorySessionStore::default()),
        threads,
    ));
    AppServer::new(
        sessions,
        Arc::new(ProviderModelService::new(Arc::new(EchoModel))),
    )
}

fn test_local_tools() -> LocalToolComposition {
    LocalToolComposition::without_executors(
        Arc::new(NoTools),
        Arc::new(RejectPolicy),
        RipgrepExecutable::from_path(std::env::current_exe().unwrap()).unwrap(),
    )
}

fn host_trust() -> WorkspaceSwitchTrustPolicy {
    WorkspaceSwitchTrustPolicy::TrustHostSelectedRoots(WorkspaceTrustSource::HostConfiguration)
}

struct RejectPolicy;

struct EmptySkillConfig;

struct TrustRevocationProvider {
    id: CloudCodeIndexProviderId,
    deletions: AtomicUsize,
    fail_deletions: AtomicBool,
}

impl TrustRevocationProvider {
    fn new() -> Self {
        Self {
            id: CloudCodeIndexProviderId::new("trust-revocation").unwrap(),
            deletions: AtomicUsize::new(0),
            fail_deletions: AtomicBool::new(false),
        }
    }
}

impl CloudCodeIndexProvider for TrustRevocationProvider {
    fn id(&self) -> &CloudCodeIndexProviderId {
        &self.id
    }

    fn capabilities(&self) -> CloudCodeIndexCapabilities {
        CloudCodeIndexCapabilities {
            deletion: CloudCodeIndexDeletionSupport::IdempotentGrantDeletion,
        }
    }

    fn publish(
        &self,
        _request: CloudCodeIndexPublicationRequest,
    ) -> Result<CloudCodeIndexPublication, CloudCodeIndexProviderError> {
        Ok(CloudCodeIndexPublication {
            remote_generation: "workspace-projection-ready".into(),
        })
    }

    fn query(
        &self,
        _request: zeta_code_index_cloud::CloudCodeIndexQueryRequest,
    ) -> Result<zeta_code_index_cloud::CloudCodeIndexQueryResult, CloudCodeIndexProviderError> {
        Err(CloudCodeIndexProviderError::new("query not configured"))
    }

    fn delete_grant(
        &self,
        _grant: &CloudCodeIndexGrant,
    ) -> Result<(), CloudCodeIndexProviderError> {
        self.deletions.fetch_add(1, Ordering::SeqCst);
        if self.fail_deletions.load(Ordering::SeqCst) {
            return Err(CloudCodeIndexProviderError::new("delete failed"));
        }
        Ok(())
    }
}

fn cloud_grant(
    controller: &zeta_code_index_cloud::CloudCodeIndexController,
) -> CloudCodeIndexGrant {
    CloudCodeIndexGrant {
        id: CloudCodeIndexGrantId::new("pending-deletion-grant").unwrap(),
        root_id: controller.root_id().as_str().to_owned(),
        destination: CloudCodeIndexDestination::new(
            CloudCodeIndexProviderId::new("trust-revocation").unwrap(),
            "tenant-a",
            "workspace-index",
        )
        .unwrap(),
        selection: CloudCodeIndexSelection::EntireIndex,
        max_egress_bytes: NonZeroU64::new(1024 * 1024).unwrap(),
    }
}

impl zeta_skills_extension::SkillConfigSnapshotProvider for EmptySkillConfig {
    fn snapshot(&self) -> Result<zeta_config::SkillsConfig, String> {
        Ok(zeta_config::SkillsConfig::default())
    }
}

impl ActionPolicyService for RejectPolicy {
    fn revision(&self) -> String {
        "test-policy-v1".into()
    }

    fn decide(
        &self,
        _: &ActionReviewRequest,
        _: &CancellationToken,
    ) -> Result<ExecutionDecision, CoreError> {
        Err(CoreError::Policy("test policy rejects every action".into()))
    }
}

static NEXT_WORKSPACE: AtomicUsize = AtomicUsize::new(0);

struct TestWorkspace {
    path: PathBuf,
}

impl TestWorkspace {
    fn new(label: &str, file: &str) -> Self {
        let sequence = NEXT_WORKSPACE.fetch_add(1, Ordering::Relaxed);
        let path = std::env::current_dir()
            .unwrap()
            .join("target")
            .join("workspace-runtime-tests")
            .join(format!("{}-{label}-{sequence}", std::process::id()));
        std::fs::create_dir_all(&path).unwrap();
        std::fs::write(path.join(file), label).unwrap();
        Self { path }
    }

    fn root(&self) -> WorkspaceRoot {
        WorkspaceRoot::open(&self.path).unwrap()
    }

    fn authorization(&self) -> WorkspaceAuthorization {
        WorkspaceAuthorization::new(
            self.root(),
            WorkspaceTrustDecision::Trusted(WorkspaceTrustSource::HostConfiguration),
        )
    }
}

impl Drop for TestWorkspace {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}
