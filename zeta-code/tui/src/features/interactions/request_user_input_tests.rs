use super::request;
use crate::components::query::QueryCustomAnswer;
use crate::features::interactions::InteractionRequest;
use zeta_protocol::RequestId;
use zeta_protocol::RequestUserInput;
use zeta_protocol::TurnId;
use zeta_protocol::UserInputOption;
use zeta_protocol::UserInputQuestion;

#[test]
fn user_input_request_preserves_pages_choices_and_custom_answer_policy() {
    let interaction = request(
        TurnId::new("turn-1").unwrap(),
        RequestId::new("request-1").unwrap(),
        RequestUserInput {
            questions: vec![UserInputQuestion {
                id: "target".into(),
                header: "Target".into(),
                question: "Which target?".into(),
                options: vec![UserInputOption {
                    label: "Library".into(),
                    description: "Build the library".into(),
                }],
                allow_free_form: true,
            }],
        },
    )
    .unwrap();

    let InteractionRequest::Query { questions, .. } = interaction else {
        panic!("expected query request");
    };
    assert_eq!(questions[0].id, "target");
    assert_eq!(questions[0].choices[0].label, "Library");
    assert_eq!(questions[0].custom_answer, QueryCustomAnswer::Allowed);
}
