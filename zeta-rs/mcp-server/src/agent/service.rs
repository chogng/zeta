use super::{
    AgentCallError, AgentEvents, AgentOutcome, AgentService, AppServerAgentService, CommandId,
    JsonRpcTransport, ReplyAgentRequest, StartAgentRequest,
};
use crate::receipt::BeginInvocation;
use sha2::{Digest, Sha256};
use std::sync::atomic::AtomicBool;

impl<T: JsonRpcTransport + Send> AgentService for AppServerAgentService<T> {
    fn start(
        &self,
        request: StartAgentRequest,
        cancellation: &AtomicBool,
        events: &dyn AgentEvents,
    ) -> Result<AgentOutcome, AgentCallError> {
        let fingerprint = super::invocation::start_fingerprint(&request);
        match self
            .receipts
            .begin(&self.principal, &request.invocation_id, fingerprint)?
        {
            BeginInvocation::Execute => {
                let result = self.start_inner(&request, cancellation, events);
                self.receipts
                    .finish(&self.principal, &request.invocation_id, fingerprint, result)
            }
            BeginInvocation::Replay(outcome) => Ok(outcome),
        }
    }

    fn reply(
        &self,
        request: ReplyAgentRequest,
        cancellation: &AtomicBool,
        events: &dyn AgentEvents,
    ) -> Result<AgentOutcome, AgentCallError> {
        let fingerprint = super::invocation::reply_fingerprint(&request);
        match self
            .receipts
            .begin(&self.principal, &request.invocation_id, fingerprint)?
        {
            BeginInvocation::Execute => {
                let result = self.reply_inner(&request, cancellation, events);
                self.receipts
                    .finish(&self.principal, &request.invocation_id, fingerprint, result)
            }
            BeginInvocation::Replay(outcome) => Ok(outcome),
        }
    }
}

pub(super) fn command_id(
    principal: &str,
    invocation_id: &str,
    operation: &str,
) -> Result<CommandId, AgentCallError> {
    let principal_digest = Sha256::digest(principal.as_bytes());
    let principal_suffix = principal_digest[..8]
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    CommandId::new(format!(
        "mcp:{principal_suffix}:{invocation_id}:{operation}"
    ))
    .map_err(|error| AgentCallError::InvalidInput(error.to_string()))
}

pub(super) fn interaction_command_id(
    principal: &str,
    invocation_id: &str,
    request_id: &zeta_protocol::RequestId,
) -> Result<CommandId, AgentCallError> {
    let digest = Sha256::digest(request_id.as_str().as_bytes());
    let suffix = digest[..8]
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    command_id(principal, invocation_id, &format!("interaction-{suffix}"))
}
