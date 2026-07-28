use super::*;
use std::collections::BTreeMap;
use std::sync::Mutex;
use zeta_async_utils::CancellationSource;
use zeta_auto_review::LlmActionClassifier;
use zeta_model_provider::ModelProviderError;
use zeta_model_provider_config::{
    ApiProfile, EndpointPolicy, Model, ModelCatalogPolicy, ModelId, ModelProviderConfig,
    ProviderAdapter, ProviderDefinition, ProviderId,
};
use zeta_policy::{
    ActionClassifier, ActionDigest, ActionKind, ActionProvenance, ActionReviewRequest,
    ActionSource, Capability, CapabilityKind, CapabilitySet, PolicyRevision, ProcessInvocationKind,
    ResolvedAction, ReviewContext, SandboxCompatibility,
};
use zeta_protocol::{ContentPart, InputItem, ModelResponse, StopReason};

struct RecordingProvider {
    selected: Arc<Mutex<Vec<ModelRef>>>,
    requests: Arc<Mutex<Vec<ModelRequest>>>,
}

impl ModelProvider for RecordingProvider {
    fn runtime(
        &self,
        request: ModelRuntimeRequest,
    ) -> Result<Arc<dyn ModelInvoker>, ModelProviderError> {
        self.selected.lock().unwrap().push(request.model);
        Ok(Arc::new(RecordingInvoker {
            requests: self.requests.clone(),
        }))
    }
}

struct RecordingInvoker {
    requests: Arc<Mutex<Vec<ModelRequest>>>,
}

impl ModelInvoker for RecordingInvoker {
    fn invoke(&self, request: &ModelRequest) -> Result<ModelResponse, ModelProviderError> {
        self.requests.lock().unwrap().push(request.clone());
        Ok(ModelResponse {
            output: vec![ResponseItem::Text(
                r#"{"recommendation":"deny","reason":"unsafe"}"#.into(),
            )],
            usage: None,
            stop_reason: StopReason::Completed,
        })
    }
}

fn provider_id(value: &str) -> ProviderId {
    ProviderId::new(value).unwrap()
}

fn model_ref(provider: &str, model: &str) -> ModelRef {
    ModelRef::new(provider_id(provider), ModelId::new(model).unwrap())
}

fn registry() -> ProviderConfigRegistry {
    ProviderConfigRegistry::from_definitions([ProviderDefinition::new(
        provider_id("test"),
        "Test",
        ProviderAdapter::OpenAiCompatible,
        ApiProfile::OpenAiChatCompletions,
        EndpointPolicy::ProviderDefault {
            base_url: "https://example.test/v1".into(),
        },
        ModelCatalogPolicy::ListedOnly,
    )
    .with_default_model(Model::new(
        ModelId::new("review-model").unwrap(),
        "Reviewer",
    ))])
    .unwrap()
}

fn review_request() -> ActionReviewRequest {
    ActionReviewRequest::new(
        ResolvedAction::new(
            ActionDigest::from_canonical_bytes(b"curl api.example.com"),
            ActionKind::LocalProcess(ProcessInvocationKind::Direct),
            "call the configured API",
            CapabilitySet::new([Capability::new(CapabilityKind::Network, "api.example.com")]),
        ),
        ActionProvenance::new(ActionSource::BuiltInTool, "shell-command"),
        SandboxCompatibility::Unsupported {
            reason: "network is unavailable".into(),
        },
        PolicyRevision::new("policy-1"),
    )
    .with_context(ReviewContext::new(
        "call the configured API for this task",
        [],
    ))
}

#[test]
fn automatic_review_resolves_provider_default_and_uses_a_review_only_request() {
    let selected = Arc::new(Mutex::new(Vec::new()));
    let requests = Arc::new(Mutex::new(Vec::new()));
    let resolver = ReviewModelResolver::new(
        registry(),
        Arc::new(RecordingProvider {
            selected: selected.clone(),
            requests: requests.clone(),
        }),
    );
    let config = ResolvedConfig {
        preferred_model: Some(model_ref("test", "agent-model")),
        providers: BTreeMap::from([(
            provider_id("test"),
            ModelProviderConfig::new(provider_id("test")),
        )]),
        ..ResolvedConfig::default()
    };

    let reviewer = resolver.resolve(&config).unwrap();
    assert_eq!(reviewer.model(), &model_ref("test", "review-model"));
    let classifier = LlmActionClassifier::new(reviewer, "review-prompt-1");
    classifier
        .classify(&review_request(), &CancellationSource::new().token())
        .unwrap();

    assert_eq!(
        selected.lock().unwrap().as_slice(),
        &[model_ref("test", "review-model")]
    );
    let requests = requests.lock().unwrap();
    let request = requests.first().unwrap();
    assert!(request.tools.is_empty());
    assert_eq!(request.tool_choice, ToolChoice::None);
    assert!(!request.parallel_tool_calls);
    let instructions = request.instructions.as_deref().unwrap();
    assert!(instructions.contains("security review classifier"));
    assert!(instructions.contains("response schema"));
    let InputItem::Message(message) = &request.input[0] else {
        panic!("review input must be a user message");
    };
    let ContentPart::Text(input) = &message.content[0] else {
        panic!("review input must be text");
    };
    assert!(input.contains("\"policy_revision\":\"policy-1\""));
    assert!(input.contains("\"user_intent\":\"call the configured API for this task\""));
}

#[test]
fn explicit_review_rejects_a_model_outside_a_listed_catalog() {
    let resolver = ReviewModelResolver::new(
        registry(),
        Arc::new(RecordingProvider {
            selected: Arc::new(Mutex::new(Vec::new())),
            requests: Arc::new(Mutex::new(Vec::new())),
        }),
    );
    let config = ResolvedConfig {
        approval_review_model: zeta_config::ApprovalReviewModelSelection::Explicit {
            model: model_ref("test", "missing"),
        },
        providers: BTreeMap::from([(
            provider_id("test"),
            ModelProviderConfig::new(provider_id("test")),
        )]),
        ..ResolvedConfig::default()
    };

    assert_eq!(
        resolver.resolve(&config).err().unwrap().to_string(),
        "model 'missing' is not registered for provider 'test'"
    );
}
