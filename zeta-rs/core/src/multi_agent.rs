//! Durable orchestration for Agent delegations across independent Threads.

mod budget;
mod context;
mod coordinator;

pub use budget::AgentTreeLimits;
pub use coordinator::CompleteDelegationRequest;
pub use coordinator::DeliveredAgentMessage;
pub use coordinator::JoinAgentsRequest;
pub use coordinator::JoinedAgents;
pub use coordinator::MultiAgentCoordinator;
pub use coordinator::SendAgentMessageRequest;
pub use coordinator::SpawnAgentRequest;
pub use coordinator::SpawnedAgent;
pub(crate) use coordinator::validate_context_seed_digest;
pub(crate) use coordinator::validate_delegation_result_digest;

pub(crate) use context::agent_context_fragments;
pub(crate) use context::scope_agent_tools;
