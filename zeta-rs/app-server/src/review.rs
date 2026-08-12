use std::fmt;
use std::sync::Arc;
use zeta_async_utils::CancellationToken;
use zeta_auto_review::LlmActionClassifier;
use zeta_auto_review::ReviewModel;
use zeta_auto_review::ReviewModelError;
use zeta_auto_review::ReviewModelRequest;
use zeta_config::ResolvedConfig;
use zeta_core::CoreError;
use zeta_core::PolicyService;
use zeta_model_provider::ModelInvoker;
use zeta_model_provider::ModelProvider;
use zeta_model_provider::ModelProviderRuntime;
use zeta_model_provider::ModelRuntimeRequest;
use zeta_model_provider_config::ProviderConfigRegistry;
use zeta_policy::ActionReviewRequest;
use zeta_policy::ExecutionDecision;
use zeta_policy::PermissionBypassGrant;
use zeta_policy::PolicyEngine;
use zeta_policy::ReviewFailurePolicy;
use zeta_protocol::ApprovalMode;
use zeta_protocol::ModelRef;
use zeta_protocol::ModelRequest;
use zeta_protocol::ResponseItem;
use zeta_protocol::ToolChoice;

/// Resolves one immutable provider-backed model for an approval assessment safe point.
///
/// Callers pass a frozen [`ResolvedConfig`] snapshot. The returned reviewer never rereads mutable
/// configuration, so a concurrent user preference change can affect only a later assessment.
pub struct ReviewModelResolver {
    registry: ProviderConfigRegistry,
    model_provider: Arc<dyn ModelProvider>,
}

impl ReviewModelResolver {
    pub fn builtin() -> Self {
        let registry = ProviderConfigRegistry::builtin();
        let model_provider = Arc::new(ModelProviderRuntime::new(registry.clone()));
        Self::new(registry, model_provider)
    }

    pub fn new(registry: ProviderConfigRegistry, model_provider: Arc<dyn ModelProvider>) -> Self {
        Self {
            registry,
            model_provider,
        }
    }

    pub fn resolve(
        &self,
        config: &ResolvedConfig,
    ) -> Result<ProviderReviewModel, ReviewModelResolutionError> {
        let model = config
            .resolve_approval_review_model(&self.registry)
            .map_err(|error| ReviewModelResolutionError(error.0))?;
        let provider = config.providers.get(&model.provider).ok_or_else(|| {
            ReviewModelResolutionError(format!(
                "approval review model provider '{}' is not configured",
                model.provider
            ))
        })?;
        let invoker = self
            .model_provider
            .runtime(ModelRuntimeRequest::new(model.clone(), provider.clone()))
            .map_err(|error| ReviewModelResolutionError(error.to_string()))?;
        Ok(ProviderReviewModel { model, invoker })
    }
}

impl Default for ReviewModelResolver {
    fn default() -> Self {
        Self::builtin()
    }
}

/// Immutable review-only adapter over one provider/model runtime.
#[derive(Clone)]
pub struct ProviderReviewModel {
    model: ModelRef,
    invoker: Arc<dyn ModelInvoker>,
}

/// Adds per-Turn approval-mode semantics around one authoritative Tool policy.
///
/// The base policy always evaluates first. Automatic review and permission bypass may replace
/// only an `AskUser` result, so deterministic denial and all request/revision validation stay
/// owned by the underlying policy.
pub(crate) struct ApprovalModePolicyService {
    base: Arc<dyn PolicyService>,
    review_model: Option<ProviderReviewModel>,
}

impl ApprovalModePolicyService {
    pub(crate) fn new(
        base: Arc<dyn PolicyService>,
        review_model: Option<ProviderReviewModel>,
    ) -> Self {
        Self { base, review_model }
    }
}

impl PolicyService for ApprovalModePolicyService {
    fn revision(&self) -> String {
        let reviewer = self.review_model.as_ref().map_or_else(
            || "unavailable".into(),
            |model| format!("{}/{}", model.model().provider, model.model().model),
        );
        format!("{}:auto-review={reviewer}", self.base.revision())
    }

    fn decide(
        &self,
        request: &ActionReviewRequest,
        cancellation: &CancellationToken,
    ) -> Result<ExecutionDecision, CoreError> {
        self.base.decide(request, cancellation)
    }

    fn decide_for_turn_with_approval_mode(
        &self,
        frozen_revision: &str,
        approval_mode: ApprovalMode,
        request: &ActionReviewRequest,
        cancellation: &CancellationToken,
    ) -> Result<ExecutionDecision, CoreError> {
        let current_revision = self.revision();
        if current_revision != frozen_revision {
            return Err(CoreError::Policy(format!(
                "Turn policy revision changed from {frozen_revision} to {current_revision}; continuation requires explicit authorization"
            )));
        }
        let decision = self.base.decide(request, cancellation)?;
        if !matches!(decision, ExecutionDecision::AskUser(_)) {
            return Ok(decision);
        }
        match approval_mode {
            ApprovalMode::AskPermissions => Ok(decision),
            ApprovalMode::BypassPermissions => Ok(ExecutionDecision::RunWithPermissionBypass(
                PermissionBypassGrant::new(
                    request.action().digest().clone(),
                    request.action().required_capabilities().clone(),
                    request.policy_revision().clone(),
                ),
            )),
            ApprovalMode::AutoReview => {
                let Some(review_model) = self.review_model.clone() else {
                    return Ok(decision);
                };
                let engine = PolicyEngine::new(
                    request.policy_revision().clone(),
                    LlmActionClassifier::new(review_model),
                    ReviewFailurePolicy::AskUser,
                );
                let reviewed = engine
                    .review_after_authoritative_ask_user(request, cancellation)
                    .map_err(|error| CoreError::Policy(error.to_string()))?;
                cancellation
                    .check()
                    .map_err(|signal| CoreError::Cancelled(signal.reason().to_string()))?;
                Ok(reviewed)
            }
        }
    }
}

impl ProviderReviewModel {
    pub fn model(&self) -> &ModelRef {
        &self.model
    }

    fn request(request: &ReviewModelRequest) -> ModelRequest {
        let mut model_request = ModelRequest::text(request.input_json());
        model_request.instructions = Some(format!(
            "{}\n\nReturn JSON matching this response schema:\n{}",
            request.system_prompt(),
            request.response_schema_json()
        ));
        model_request.tools.clear();
        model_request.tool_choice = ToolChoice::None;
        model_request.parallel_tool_calls = false;
        model_request.temperature = Some(0.0);
        model_request
    }
}

impl ReviewModel for ProviderReviewModel {
    fn complete(
        &self,
        request: &ReviewModelRequest,
        cancellation: &CancellationToken,
    ) -> Result<String, ReviewModelError> {
        cancellation
            .check()
            .map_err(|signal| ReviewModelError::Invocation(signal.reason().to_string()))?;
        let response = self
            .invoker
            .invoke(&Self::request(request))
            .map_err(|error| ReviewModelError::Invocation(error.to_string()))?;
        cancellation
            .check()
            .map_err(|signal| ReviewModelError::Invocation(signal.reason().to_string()))?;

        let mut text = String::new();
        for item in response.output {
            match item {
                ResponseItem::Text(fragment) => {
                    let bytes = text
                        .len()
                        .checked_add(fragment.len())
                        .ok_or(ReviewModelError::ResponseTooLarge { bytes: usize::MAX })?;
                    if bytes > request.maximum_response_bytes() {
                        return Err(ReviewModelError::ResponseTooLarge { bytes });
                    }
                    text.push_str(&fragment);
                }
                ResponseItem::Reasoning(_) => {}
                ResponseItem::Refusal(reason) => {
                    return Err(ReviewModelError::Invocation(format!(
                        "review model refused the assessment: {reason}"
                    )));
                }
                ResponseItem::ToolCall(_) => {
                    return Err(ReviewModelError::Invocation(
                        "review model attempted a tool call".into(),
                    ));
                }
            }
        }
        if text.trim().is_empty() {
            return Err(ReviewModelError::Invocation(
                "review model returned no JSON response".into(),
            ));
        }
        Ok(text)
    }
}

/// Failure to materialize a configured approval review model.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReviewModelResolutionError(String);

impl fmt::Display for ReviewModelResolutionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for ReviewModelResolutionError {}

#[cfg(test)]
#[path = "review_tests.rs"]
mod tests;
