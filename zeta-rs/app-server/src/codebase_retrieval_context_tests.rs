use std::sync::Arc;

use tempfile::TempDir;
use zeta_async_utils::CancellationSource;
use zeta_codebase::Codebase;
use zeta_codebase::CodebaseLimits;
use zeta_codebase::CodebaseSemanticService;
use zeta_codebase::EmbeddingIndexKey;
use zeta_codebase::InMemoryCodebaseVectorStore;
use zeta_config::CodebaseAutomaticContext;
use zeta_config::CodebaseModelSelection;
use zeta_config::ConfigCommandRequest;
use zeta_config::ConfigRevision;
use zeta_config::ConfigStore;
use zeta_config::UserConfigCommand;
use zeta_core::ContextSource;
use zeta_core::ContextSourceRequest;
use zeta_file_access::Dir;
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

use super::CodebaseRetrievalContextSource;

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
fn automatic_context_requires_explicit_opt_in() {
    let dir = dir();
    let root = Dir::open_local(dir.path()).expect("directory root");
    let index = Arc::new(Codebase::open_memory(root, CodebaseLimits::default()).expect("Codebase"));
    index.rebuild().expect("lexical index");
    let semantic = Arc::new(CodebaseSemanticService::new(
        Arc::clone(&index),
        EmbeddingIndexKey::new("test-embedding").expect("model id"),
        Arc::new(ConstantEmbedding),
        Arc::new(InMemoryCodebaseVectorStore::default()),
    ));
    semantic.sync().expect("semantic projection");
    let profile = tempfile::tempdir().expect("profile");
    let config =
        Arc::new(ConfigStore::open(profile.path().join("config.sqlite3")).expect("config"));
    let source = CodebaseRetrievalContextSource::new(
        Arc::clone(&index),
        None,
        Some(semantic),
        None,
        Some(Arc::clone(&config)),
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
    config
        .apply(ConfigCommandRequest {
            command_id: CommandId::new("configure-semantic").expect("command id"),
            expected_revision: configured_provider.revision,
            command: UserConfigCommand::ConfigureCodebase {
                models: Some(CodebaseModelSelection {
                    embedding_model: ModelRef::new(
                        provider,
                        ModelId::new("embedding-v1").expect("model id"),
                    ),
                    rerank_model: None,
                }),
                automatic_context: CodebaseAutomaticContext::FirstInvocation,
            },
        })
        .expect("configure semantic context");
    let evidence = source
        .collect(&request, &cancellation.token())
        .expect("enabled context");

    assert_eq!(evidence.len(), 1);
    assert_eq!(evidence[0].source, "codebase");
    assert!(evidence[0].reference.starts_with("lib.rs:"));
    assert!(evidence[0].body.contains("target_feature"));
}

fn dir() -> TempDir {
    let dir = tempfile::tempdir().expect("directory");
    std::fs::create_dir(dir.path().join(".git")).expect("git marker");
    std::fs::write(
        dir.path().join("lib.rs"),
        "pub fn target_feature() -> bool { true }\n",
    )
    .expect("source");
    dir
}
