use super::*;
use zeta_protocol::{RequestUserInput, UserInputQuestion};

#[test]
fn form_elicitation_rejects_questions_that_request_sensitive_information() {
    let request = RequestUserInput {
        questions: vec![UserInputQuestion {
            id: "api_key".into(),
            header: "Authentication".into(),
            question: "Enter the value".into(),
            options: Vec::new(),
            allow_free_form: true,
        }],
    };

    assert!(!user_input_is_form_safe(&request));
}

#[test]
fn form_elicitation_accepts_non_sensitive_user_questions() {
    let request = RequestUserInput {
        questions: vec![UserInputQuestion {
            id: "test_scope".into(),
            header: "Test scope".into(),
            question: "Which test suite should Zeta run?".into(),
            options: Vec::new(),
            allow_free_form: true,
        }],
    };

    assert!(user_input_is_form_safe(&request));
}
