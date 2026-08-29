use super::InteractionBinding;
use super::InteractionRequest;
use crate::components::query::QueryChoice;
use crate::components::query::QueryCustomAnswer;
use crate::components::query::QueryQuestion;
use zeta_protocol::RequestId;
use zeta_protocol::RequestUserInput;
use zeta_protocol::TurnId;

pub(super) fn request(
    turn_id: TurnId,
    request_id: RequestId,
    request: RequestUserInput,
) -> Result<InteractionRequest, String> {
    let questions = request
        .questions
        .into_iter()
        .map(|question| QueryQuestion {
            id: question.id,
            header: question.header,
            prompt: question.question,
            choices: question
                .options
                .into_iter()
                .map(|option| QueryChoice {
                    label: option.label,
                    description: option.description,
                })
                .collect(),
            custom_answer: if question.allow_free_form {
                QueryCustomAnswer::Allowed
            } else {
                QueryCustomAnswer::Unavailable
            },
        })
        .collect::<Vec<_>>();
    if questions.is_empty() {
        return Err("a user-input request requires at least one question".into());
    }
    Ok(InteractionRequest::Query {
        binding: InteractionBinding::Query {
            turn_id,
            request_id,
        },
        questions,
    })
}

#[cfg(test)]
#[path = "request_user_input_tests.rs"]
mod tests;
