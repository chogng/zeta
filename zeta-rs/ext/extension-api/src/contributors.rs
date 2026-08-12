use crate::ExtensionError;
use crate::PromptFragment;
use std::sync::Arc;
use zeta_protocol::FrozenSkillActivation;
use zeta_protocol::ThreadId;
use zeta_protocol::TurnId;
use zeta_protocol::UserInput;
use zeta_tools::ToolExecutor;

/// Immutable user input available while a new Turn's capability activations are resolved.
pub struct SkillActivationContext<'a> {
    user_input: &'a [UserInput],
}

impl<'a> SkillActivationContext<'a> {
    pub fn new(user_input: &'a [UserInput]) -> Self {
        Self { user_input }
    }

    pub fn user_input(&self) -> &'a [UserInput] {
        self.user_input
    }
}

/// Resolves Skill selections into exact durable activations before Core accepts a new Turn.
///
/// Implementations must preserve user-visible selection order, enforce their own availability and
/// compatibility policy, and return content-bound activations. They must not mutate Thread state.
pub trait SkillActivationContributor: Send + Sync {
    fn contribute(
        &self,
        input: SkillActivationContext<'_>,
    ) -> Result<Vec<FrozenSkillActivation>, ExtensionError>;
}

/// Immutable facts exposed at one model-invocation safe point.
pub struct TurnInputContext<'a> {
    thread_id: &'a ThreadId,
    turn_id: &'a TurnId,
    activated_skills: &'a [FrozenSkillActivation],
}

impl<'a> TurnInputContext<'a> {
    pub fn new(
        thread_id: &'a ThreadId,
        turn_id: &'a TurnId,
        activated_skills: &'a [FrozenSkillActivation],
    ) -> Self {
        Self {
            thread_id,
            turn_id,
            activated_skills,
        }
    }

    pub fn thread_id(&self) -> &'a ThreadId {
        self.thread_id
    }

    pub fn turn_id(&self) -> &'a TurnId {
        self.turn_id
    }

    pub fn activated_skills(&self) -> &'a [FrozenSkillActivation] {
        self.activated_skills
    }
}

/// Contributes immutable prompt fragments for one model invocation.
///
/// Implementations own domain loading and policy. Core owns precedence, budgeting, and final model
/// request assembly after validating the returned fragment metadata.
pub trait TurnInputContributor: Send + Sync {
    fn contribute(
        &self,
        input: TurnInputContext<'_>,
    ) -> Result<Vec<PromptFragment>, ExtensionError>;
}

/// Contributes authority-free, read-only model tools from a shared agent extension.
///
/// Implementations may read immutable or internally synchronized extension-owned state, including
/// source roots that the extension validated before registration. They must not use ambient
/// filesystem authority, mutate external state, start processes, access the network or credentials,
/// or append durable Thread events. The installing host validates and routes these executors through
/// its normal tool policy boundary. Extensions that need broader authority must use a
/// capability-bearing host contract instead of this one.
pub trait ReadOnlyToolContributor: Send + Sync {
    fn contribute(&self) -> Result<Vec<Arc<dyn ToolExecutor>>, ExtensionError>;
}

/// Exact external authority declared by a capability-bearing extension tool.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ExtensionToolAuthority {
    /// Sends read-only requests to explicit network scopes without mutating the remote service.
    ExternalRead {
        service: String,
        network_scopes: Vec<String>,
        credential_reference: Option<String>,
    },
}

/// One executable extension tool paired with the authority its host policy must review.
pub struct CapabilityToolContribution {
    executor: Arc<dyn ToolExecutor>,
    authority: ExtensionToolAuthority,
}

impl CapabilityToolContribution {
    pub fn new(executor: Arc<dyn ToolExecutor>, authority: ExtensionToolAuthority) -> Self {
        Self {
            executor,
            authority,
        }
    }

    pub fn executor(&self) -> &Arc<dyn ToolExecutor> {
        &self.executor
    }

    pub fn authority(&self) -> &ExtensionToolAuthority {
        &self.authority
    }

    pub fn into_parts(self) -> (Arc<dyn ToolExecutor>, ExtensionToolAuthority) {
        (self.executor, self.authority)
    }
}

/// Contributes tools that need explicit host-reviewed network or credential authority.
///
/// Implementations must declare every external scope before registration. The host freezes the
/// declaration with the Tool Call, asks policy for authority, and only then invokes the executor.
/// This contract does not itself grant network or credential access.
pub trait CapabilityToolContributor: Send + Sync {
    fn contribute(&self) -> Result<Vec<CapabilityToolContribution>, ExtensionError>;
}
