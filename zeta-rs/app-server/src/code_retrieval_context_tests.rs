use std::sync::Arc;

use tempfile::TempDir;
use zeta_async_utils::CancellationSource;
use zeta_code_index::CodeIndex;
use zeta_code_index::CodeIndexLimits;
use zeta_code_index::CodeIndexStorage;
use zeta_code_index_semantic::CodeIndexEmbeddingModelId;
use zeta_code_index_semantic::CodeIndexSemanticService;
use zeta_code_index_semantic::InMemoryCodeIndexVectorStore;
use zeta_config::ConfigCommandRequest;
use zeta_config::ConfigRevision;
use zeta_config::ConfigStore;
use zeta_config::SemanticCodeIndexAutomaticContext;
use zeta_config::SemanticCodeIndexModelSelection;
use zeta_config::SemanticCodeIndexSelection;
use zeta_config::UserConfigCommand;
use zeta_core::ContextSource;
use zeta_core::ContextSourceRequest;
use zeta_model_provider::EmbeddingInvoker;
use zeta_model_provider::EmbeddingRequest;
use zeta_model_provider::EmbeddingResponse;
use zeta_model_provider::EmbeddingVector;
use zeta_model_provider::ModelProviderError;
use zeta_model_provider_config::ModelProviderConfig;
use zeta_protocol::CommandId;
use zeta_protocol::ModelId;
use zeta_protocol::ModelRef;
use zeta_protocol::ProviderId;
use zeta_protocol::SessionId;
use zeta_protocol::ThreadId;
use zeta_protocol::TurnId;
use zeta_workspace::WorkspaceRoot;

use super::CodeRetrievalContextSource;

struct ConstantEmbedding;

impl EmbeddingInvoker for ConstantEmbedding {
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
fn automatic_context_requires_both_opt_in_and_current_workspace_egress_consent() {
    let workspace = workspace();
    let root = WorkspaceRoot::open(workspace.path()).expect("workspace root");
    let workspace_id = root.trust_id();
    let index = Arc::new(
        CodeIndex::open(root, CodeIndexStorage::Memory, CodeIndexLimits::default())
            .expect("code index"),
    );
    index.rebuild().expect("lexical index");
    let semantic = Arc::new(CodeIndexSemanticService::new(
        Arc::clone(&index),
        CodeIndexEmbeddingModelId::new("test-embedding").expect("model id"),
        Arc::new(ConstantEmbedding),
        Arc::new(InMemoryCodeIndexVectorStore::default()),
    ));
    semantic.sync().expect("semantic projection");
    let profile = tempfile::tempdir().expect("profile");
    let config =
        Arc::new(ConfigStore::open(profile.path().join("config.sqlite3")).expect("config"));
    let source = CodeRetrievalContextSource::new(
        Arc::clone(&index),
        None,
        Some(semantic),
        None,
        Some(Arc::clone(&config)),
        workspace_id.clone(),
    );
    let session_id = SessionId::new("session-1").expect("session id");
    let thread_id = ThreadId::new("thread-1").expect("thread id");
    let turn_id = TurnId::new("turn-1").expect("turn id");
    let request = ContextSourceRequest {
        session_id: &session_id,
        thread_id: &thread_id,
        turn_id: &turn_id,
        query: "find the target feature",
    };
    let cancellation = CancellationSource::new();

    assert!(
        source
            .collect(&request, &cancellation.token())
            .expect("default context")
            .is_empty()
    );

    let provider = ProviderId::new("openai-compatible").expect("provider");
    let configured_provider = config
        .apply(ConfigCommandRequest {
            command_id: CommandId::new("configure-provider").expect("command id"),
            expected_revision: ConfigRevision::INITIAL,
            command: UserConfigCommand::ConfigureProvider {
                provider: provider.clone(),
                config: ModelProviderConfig::new(provider.clone()),
            },
        })
        .expect("configure provider");
    let configured_semantic = config
        .apply(ConfigCommandRequest {
            command_id: CommandId::new("configure-semantic").expect("command id"),
            expected_revision: configured_provider.revision,
            command: UserConfigCommand::ConfigureSemanticCodeIndex {
                selection: SemanticCodeIndexSelection::Remote {
                    models: SemanticCodeIndexModelSelection {
                        embedding_model: ModelRef::new(
                            provider,
                            ModelId::new("embedding-v1").expect("model id"),
                        ),
                        rerank_model: None,
                    },
                },
                automatic_context: SemanticCodeIndexAutomaticContext::FirstInvocation,
            },
        })
        .expect("configure semantic context");
    assert!(
        source
            .collect(&request, &cancellation.token())
            .expect("unapproved context")
            .is_empty()
    );

    config
        .apply(ConfigCommandRequest {
            command_id: CommandId::new("authorize-semantic").expect("command id"),
            expected_revision: configured_semantic.revision,
            command: UserConfigCommand::AuthorizeSemanticCodeIndexEgress {
                workspace: workspace_id,
            },
        })
        .expect("authorize workspace");
    let evidence = source
        .collect(&request, &cancellation.token())
        .expect("authorized context");

    assert_eq!(evidence.len(), 1);
    assert_eq!(evidence[0].source, "code-index");
    assert!(evidence[0].reference.starts_with("lib.rs:"));
    assert!(evidence[0].body.contains("target_feature"));
}

fn workspace() -> TempDir {
    let workspace = tempfile::tempdir().expect("workspace");
    std::fs::create_dir(workspace.path().join(".git")).expect("git marker");
    std::fs::write(
        workspace.path().join("lib.rs"),
        "pub fn target_feature() -> bool { true }\n",
    )
    .expect("source");
    workspace
}
