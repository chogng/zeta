use zeta_protocol::ContextWindow;
use zeta_protocol::Model;
use zeta_protocol::ModelAccess;
use zeta_protocol::ModelCapabilities;
use zeta_protocol::ModelId;
use zeta_protocol::ModelRef;
use zeta_protocol::Personality;
use zeta_protocol::ProviderId;
use zeta_protocol::ReasoningEffort;

/// Product execution surface selected for one built-in model row.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StaticModelRuntime {
    ProviderApi,
    ChatGptSubscription,
    KimiCode,
}

/// One row in Zeta's product-level static model catalog.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StaticModelSpec {
    pub provider_id: &'static str,
    pub model_id: &'static str,
    pub display_name: &'static str,
    pub access: ModelAccess,
    pub runtime: StaticModelRuntime,
    pub context_window: ContextWindow,
    pub auto_compact_token_limit: Option<u32>,
    pub capabilities: ModelCapabilities,
    pub supported_reasoning_efforts: &'static [ReasoningEffort],
    pub default_reasoning_effort: Option<ReasoningEffort>,
    pub default_personality: Option<Personality>,
    pub supports_input_token_count: bool,
    pub is_approval_review_default: bool,
}

impl StaticModelSpec {
    /// Builds the runtime model identity represented by this static row.
    pub fn model_ref(&self) -> ModelRef {
        ModelRef::new(
            ProviderId::new(self.provider_id).expect("static provider ID is valid"),
            ModelId::new(self.model_id).expect("static model ID is valid"),
        )
    }

    /// Builds provider-neutral model metadata from this static row.
    pub fn model(&self) -> Model {
        let mut model = Model::new(
            ModelId::new(self.model_id).expect("static model ID is valid"),
            self.display_name,
        );
        model.context_window = self.context_window;
        model.access = self.access;
        model.auto_compact_token_limit = self.auto_compact_token_limit;
        model.capabilities = self.capabilities;
        model.supported_reasoning_efforts = self.supported_reasoning_efforts.to_vec();
        model.default_reasoning_effort = self.default_reasoning_effort;
        model.default_personality = self.default_personality;
        model
    }

    /// Whether the declared context window is exactly one million tokens.
    pub fn has_one_million_context(&self) -> bool {
        self.context_window == ContextWindow::Known(1_000_000)
    }
}

macro_rules! static_model {
    {
        provider: $provider:expr,
        id: $model:expr,
        name: $name:expr,
        access: $access:ident,
        $(runtime: $runtime:ident,)?
        $(context_window: $context_window:expr,)?
        $(auto_compact_token_limit: $auto_compact_token_limit:expr,)?
        $(capabilities: {
            $($capability:ident: $support:ident),* $(,)?
        },)?
        $(reasoning: [$($reasoning:ident),* $(,)?],)?
        $(default_reasoning: $default_reasoning:ident,)?
        $(default_personality: $default_personality:ident,)?
        $(input_token_count: $input_token_count:literal,)?
        $(approval_review_default: $approval_review_default:literal,)?
    } => {
        $crate::static_model_spec::StaticModelSpec {
            provider_id: $provider,
            model_id: $model,
            display_name: $name,
            access: static_model!(@access $access),
            runtime: static_model!(@runtime $($runtime)?),
            context_window: static_model!(@context_window $($context_window)?),
            auto_compact_token_limit: static_model!(@optional_u32 $($auto_compact_token_limit)?),
            capabilities: zeta_protocol::ModelCapabilities {
                $($(
                    $capability: static_model!(@support $support),
                )*)?
                ..zeta_protocol::ModelCapabilities::UNKNOWN
            },
            supported_reasoning_efforts: &[$($(static_model!(@reasoning $reasoning)),*)?],
            default_reasoning_effort: static_model!(@optional_reasoning $($default_reasoning)?),
            default_personality: static_model!(@optional_personality $($default_personality)?),
            supports_input_token_count: static_model!(@bool $($input_token_count)?),
            is_approval_review_default: static_model!(@bool $($approval_review_default)?),
        }
    };

    (@access api_key) => { zeta_protocol::ModelAccess::ApiKey };
    (@access subscription) => { zeta_protocol::ModelAccess::Subscription };
    (@access local) => { zeta_protocol::ModelAccess::Local };
    (@access enterprise) => { zeta_protocol::ModelAccess::Enterprise };
    (@access unknown) => { zeta_protocol::ModelAccess::Unknown };

    (@runtime) => { $crate::static_model_spec::StaticModelRuntime::ProviderApi };
    (@runtime provider_api) => { $crate::static_model_spec::StaticModelRuntime::ProviderApi };
    (@runtime chatgpt_subscription) => { $crate::static_model_spec::StaticModelRuntime::ChatGptSubscription };
    (@runtime kimi_code) => { $crate::static_model_spec::StaticModelRuntime::KimiCode };

    (@context_window) => { zeta_protocol::ContextWindow::Unknown };
    (@context_window $tokens:expr) => { zeta_protocol::ContextWindow::Known($tokens) };
    (@optional_u32) => { None };
    (@optional_u32 $value:expr) => { Some($value) };
    (@bool) => { false };
    (@bool $value:literal) => { $value };

    (@support supported) => { zeta_protocol::CapabilitySupport::Supported };
    (@support unsupported) => { zeta_protocol::CapabilitySupport::Unsupported };
    (@support unknown) => { zeta_protocol::CapabilitySupport::Unknown };

    (@reasoning none) => { zeta_protocol::ReasoningEffort::None };
    (@reasoning minimal) => { zeta_protocol::ReasoningEffort::Minimal };
    (@reasoning low) => { zeta_protocol::ReasoningEffort::Low };
    (@reasoning medium) => { zeta_protocol::ReasoningEffort::Medium };
    (@reasoning high) => { zeta_protocol::ReasoningEffort::High };
    (@reasoning extra_high) => { zeta_protocol::ReasoningEffort::ExtraHigh };
    (@reasoning max) => { zeta_protocol::ReasoningEffort::Max };
    (@optional_reasoning) => { None };
    (@optional_reasoning $value:ident) => { Some(static_model!(@reasoning $value)) };

    (@personality friendly) => { zeta_protocol::Personality::Friendly };
    (@personality pragmatic) => { zeta_protocol::Personality::Pragmatic };
    (@personality none) => { zeta_protocol::Personality::None };
    (@optional_personality) => { None };
    (@optional_personality $value:ident) => { Some(static_model!(@personality $value)) };
}

pub(crate) use static_model;

#[cfg(test)]
mod tests {
    use super::*;
    use zeta_protocol::CapabilitySupport;

    const COMPLETE_SPEC: StaticModelSpec = static_model! {
        provider: "test-provider",
        id: "test-model",
        name: "Test Model",
        access: subscription,
        runtime: kimi_code,
        context_window: 1_000_000,
        auto_compact_token_limit: 900_000,
        capabilities: {
            tools: supported,
            reasoning: unsupported,
        },
        reasoning: [medium, high],
        default_reasoning: high,
        default_personality: pragmatic,
        input_token_count: true,
        approval_review_default: true,
    };

    #[test]
    fn declaration_macro_applies_named_fields_and_explicit_defaults() {
        assert_eq!(COMPLETE_SPEC.access, ModelAccess::Subscription);
        assert_eq!(COMPLETE_SPEC.runtime, StaticModelRuntime::KimiCode);
        assert_eq!(
            COMPLETE_SPEC.context_window,
            ContextWindow::Known(1_000_000)
        );
        assert_eq!(COMPLETE_SPEC.auto_compact_token_limit, Some(900_000));
        assert_eq!(
            COMPLETE_SPEC.capabilities.tools,
            CapabilitySupport::Supported
        );
        assert_eq!(
            COMPLETE_SPEC.capabilities.reasoning,
            CapabilitySupport::Unsupported
        );
        assert_eq!(
            COMPLETE_SPEC.supported_reasoning_efforts,
            &[ReasoningEffort::Medium, ReasoningEffort::High]
        );
        assert_eq!(
            COMPLETE_SPEC.default_reasoning_effort,
            Some(ReasoningEffort::High)
        );
        assert_eq!(
            COMPLETE_SPEC.default_personality,
            Some(Personality::Pragmatic)
        );
        assert!(COMPLETE_SPEC.supports_input_token_count);
        assert!(COMPLETE_SPEC.is_approval_review_default);
    }
}
