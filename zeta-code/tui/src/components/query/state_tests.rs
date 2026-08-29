use super::Query;
use super::QueryChoice;
use super::QueryCustomAnswer;
use super::QueryOutcome;
use super::QueryQuestion;

#[test]
fn fixed_answers_advance_pages_and_complete_once() {
    let mut query = Query::new(vec![question("one"), question("two")]).unwrap();

    assert_eq!(query.activate(0), Some(QueryOutcome::Consumed));
    let outcome = query.activate(0).unwrap();

    let QueryOutcome::Completed(answers) = outcome else {
        panic!("expected completed query");
    };
    assert_eq!(answers[0].question_id, "one");
    assert_eq!(answers[1].question_id, "two");
    assert_eq!(answers[0].value, "Yes");
    assert!(query.activate(0).is_none());
}

#[test]
fn custom_answer_keeps_the_question_until_text_is_submitted() {
    let mut query = Query::new(vec![QueryQuestion {
        id: "custom".into(),
        header: "Choice".into(),
        prompt: "What should happen?".into(),
        choices: vec![QueryChoice {
            label: "Default".into(),
            description: "Use the default".into(),
        }],
        custom_answer: QueryCustomAnswer::Allowed,
    }])
    .unwrap();

    assert_eq!(query.activate(1), Some(QueryOutcome::BeginCustomAnswer));
    assert_eq!(query.view().current, 0);
    let QueryOutcome::Completed(answers) = query.submit_custom_answer("Custom".into()) else {
        panic!("expected completed query");
    };
    assert_eq!(answers[0].value, "Custom");
}

fn question(id: &str) -> QueryQuestion {
    QueryQuestion {
        id: id.into(),
        header: "Confirm".into(),
        prompt: "Continue?".into(),
        choices: vec![QueryChoice {
            label: "Yes".into(),
            description: "Continue".into(),
        }],
        custom_answer: QueryCustomAnswer::Unavailable,
    }
}
