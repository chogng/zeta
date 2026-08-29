//! Adapts protocol Agent interactions into explicit TUI approval and query components.

mod approval;
mod request_user_input;

use crate::components::approval::ApprovalDecision;
use crate::components::approval::ApprovalSpec;
use crate::components::chat_input_area::ChatInputAreaInteractionId;
use crate::components::query::QueryAnswer;
use crate::components::query::QueryQuestion;
use zeta_protocol::ActionApprovalDecision;
use zeta_protocol::ActionApprovalResponse;
use zeta_protocol::AgentRequest;
use zeta_protocol::AgentRequestEnvelope;
use zeta_protocol::AgentResponse;
use zeta_protocol::RequestId;
use zeta_protocol::RequestUserInputResponse;
use zeta_protocol::TurnId;
use zeta_protocol::UserInputAnswer;

pub(crate) enum InteractionRequest {
    Approval {
        binding: InteractionBinding,
        spec: ApprovalSpec,
    },
    Query {
        binding: InteractionBinding,
        questions: Vec<QueryQuestion>,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct InteractionResponse {
    pub(crate) interaction_id: ChatInputAreaInteractionId,
    pub(crate) turn_id: TurnId,
    pub(crate) request_id: RequestId,
    pub(crate) response: AgentResponse,
}

#[derive(Clone, Debug)]
pub(crate) enum InteractionBinding {
    Approval {
        turn_id: TurnId,
        request_id: RequestId,
    },
    Query {
        turn_id: TurnId,
        request_id: RequestId,
    },
}

impl InteractionBinding {
    pub(crate) fn matches_request(&self, turn_id: &TurnId, request_id: &RequestId) -> bool {
        match self {
            Self::Approval {
                turn_id: bound_turn,
                request_id: bound_request,
            }
            | Self::Query {
                turn_id: bound_turn,
                request_id: bound_request,
            } => bound_turn == turn_id && bound_request == request_id,
        }
    }

    pub(crate) fn approval_response(
        &self,
        interaction_id: ChatInputAreaInteractionId,
        decision: ApprovalDecision,
    ) -> Result<InteractionResponse, String> {
        let Self::Approval {
            turn_id,
            request_id,
        } = self
        else {
            return Err("a query binding cannot resolve an approval".into());
        };
        let decision = match decision {
            ApprovalDecision::ApproveOnce => ActionApprovalDecision::ApproveOnce,
            ApprovalDecision::Decline => ActionApprovalDecision::Decline,
        };
        Ok(InteractionResponse {
            interaction_id,
            turn_id: turn_id.clone(),
            request_id: request_id.clone(),
            response: AgentResponse::Approval {
                response: ActionApprovalResponse { decision },
            },
        })
    }

    pub(crate) fn query_response(
        &self,
        interaction_id: ChatInputAreaInteractionId,
        answers: Vec<QueryAnswer>,
    ) -> Result<InteractionResponse, String> {
        let Self::Query {
            turn_id,
            request_id,
        } = self
        else {
            return Err("an approval binding cannot resolve a query".into());
        };
        let answers = answers
            .into_iter()
            .map(|answer| {
                (
                    answer.question_id,
                    UserInputAnswer {
                        value: answer.value,
                    },
                )
            })
            .collect();
        Ok(InteractionResponse {
            interaction_id,
            turn_id: turn_id.clone(),
            request_id: request_id.clone(),
            response: AgentResponse::UserInput {
                response: RequestUserInputResponse { answers },
            },
        })
    }
}

pub(crate) fn interaction_request(
    envelope: AgentRequestEnvelope,
) -> Result<InteractionRequest, String> {
    let turn_id = envelope.turn_id;
    let request_id = envelope.interaction.request_id;
    match envelope.interaction.request {
        AgentRequest::Approval { request } => Ok(approval::request(turn_id, request_id, request)),
        AgentRequest::UserInput { request } => {
            request_user_input::request(turn_id, request_id, request)
        }
        AgentRequest::DynamicTool { .. } => {
            Err("dynamic Tool interaction is not supported by this TUI".into())
        }
    }
}
