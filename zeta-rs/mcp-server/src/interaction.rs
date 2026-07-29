use serde::Deserialize;
use serde_json::{Map, Value, json};
use std::collections::BTreeMap;
use zeta_protocol::{
    ActionApprovalDecision, ActionApprovalResponse, AgentRequest, AgentRequestEnvelope,
    AgentResponse, RequestUserInput, RequestUserInputResponse, UserInputAnswer,
};

const MAX_ELICITATION_MESSAGE_BYTES: usize = 16 * 1024;

pub(crate) fn elicitation_params(request: &AgentRequestEnvelope) -> Option<Value> {
    match &request.interaction.request {
        AgentRequest::Approval { request } => {
            let capabilities = request
                .capabilities
                .iter()
                .map(|capability| format!("{:?}: {}", capability.kind, capability.scope))
                .collect::<Vec<_>>()
                .join(", ");
            Some(json!({
                "mode": "form",
                "message": truncate(format!(
                    "{}\nRequested capabilities: {}",
                    request.reason, capabilities
                )),
                "requestedSchema": {
                    "type": "object",
                    "properties": {
                        "decision": {
                            "type": "string",
                            "title": "Approval decision",
                            "description": "Approve this exact action once, or decline it.",
                            "oneOf": [
                                {"const": "approveOnce", "title": "Approve once"},
                                {"const": "decline", "title": "Decline"}
                            ]
                        }
                    },
                    "required": ["decision"]
                }
            }))
        }
        AgentRequest::UserInput { request } => {
            if !user_input_is_form_safe(request) {
                return None;
            }
            let mut properties = Map::new();
            let mut required = Vec::new();
            for question in &request.questions {
                let mut schema = Map::from_iter([
                    ("type".into(), Value::String("string".into())),
                    ("title".into(), Value::String(question.header.clone())),
                    (
                        "description".into(),
                        Value::String(truncate(question.question.clone())),
                    ),
                ]);
                if !question.allow_free_form && !question.options.is_empty() {
                    schema.insert(
                        "oneOf".into(),
                        Value::Array(
                            question
                                .options
                                .iter()
                                .map(|option| {
                                    json!({
                                        "const": option.label,
                                        "title": option.label
                                    })
                                })
                                .collect(),
                        ),
                    );
                }
                properties.insert(question.id.clone(), Value::Object(schema));
                required.push(Value::String(question.id.clone()));
            }
            Some(json!({
                "mode": "form",
                "message": "Zeta needs additional information to continue this task.",
                "requestedSchema": {
                    "type": "object",
                    "properties": properties,
                    "required": required
                }
            }))
        }
        AgentRequest::DynamicTool { .. } => None,
    }
}

fn user_input_is_form_safe(request: &RequestUserInput) -> bool {
    request.questions.iter().all(|question| {
        [
            question.id.as_str(),
            question.header.as_str(),
            question.question.as_str(),
        ]
        .into_iter()
        .chain(
            question
                .options
                .iter()
                .flat_map(|option| [option.label.as_str(), option.description.as_str()]),
        )
        .all(|text| !looks_like_sensitive_information(text))
    })
}

fn looks_like_sensitive_information(value: &str) -> bool {
    const SENSITIVE_TERMS: &[&str] = &[
        "password",
        "passcode",
        "api key",
        "apikey",
        "access token",
        "refresh token",
        "auth token",
        "bearer token",
        "secret",
        "credential",
        "credit card",
        "card number",
        "cvv",
        "private key",
        "seed phrase",
        "recovery phrase",
        "密码",
        "密钥",
        "令牌",
        "凭证",
        "信用卡",
    ];
    let normalized = value.to_lowercase().replace(['_', '-'], " ");
    SENSITIVE_TERMS.iter().any(|term| normalized.contains(term))
}

pub(crate) fn decode_elicitation_result(
    request: &AgentRequestEnvelope,
    value: Value,
) -> Option<AgentResponse> {
    let result: ElicitationResult = serde_json::from_value(value).ok()?;
    if result.action != ElicitationAction::Accept {
        return None;
    }
    let content = result.content?;
    match &request.interaction.request {
        AgentRequest::Approval { .. } => {
            let decision = match content.get("decision")?.as_str()? {
                "approveOnce" => ActionApprovalDecision::ApproveOnce,
                "decline" => ActionApprovalDecision::Decline,
                _ => return None,
            };
            Some(AgentResponse::Approval {
                response: ActionApprovalResponse { decision },
            })
        }
        AgentRequest::UserInput { request } => {
            let mut answers = BTreeMap::new();
            for question in &request.questions {
                let value = content.get(&question.id)?.as_str()?.to_string();
                answers.insert(question.id.clone(), UserInputAnswer { value });
            }
            Some(AgentResponse::UserInput {
                response: RequestUserInputResponse { answers },
            })
        }
        AgentRequest::DynamicTool { .. } => None,
    }
}

#[derive(Deserialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
enum ElicitationAction {
    Accept,
    Decline,
    Cancel,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ElicitationResult {
    action: ElicitationAction,
    content: Option<Map<String, Value>>,
}

fn truncate(mut value: String) -> String {
    if value.len() <= MAX_ELICITATION_MESSAGE_BYTES {
        return value;
    }
    let mut boundary = MAX_ELICITATION_MESSAGE_BYTES;
    while !value.is_char_boundary(boundary) {
        boundary -= 1;
    }
    value.truncate(boundary);
    value
}

#[cfg(test)]
#[path = "interaction_tests.rs"]
mod tests;
