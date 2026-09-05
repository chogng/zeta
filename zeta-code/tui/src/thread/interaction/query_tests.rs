use super::Query;
use super::QueryChoice;
use super::QueryCustomAnswer;
use super::QueryOutcome;
use super::QueryQuestion;
use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use crossterm::event::KeyModifiers;

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

    assert_eq!(query.activate(1), Some(QueryOutcome::Consumed));
    assert_eq!(query.view().current, 0);
    for character in "jk/i p".chars() {
        assert_eq!(
            query.handle_key(KeyEvent::new(KeyCode::Char(character), KeyModifiers::NONE,)),
            QueryOutcome::Consumed
        );
    }
    let QueryOutcome::Completed(answers) =
        query.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE))
    else {
        panic!("expected completed query");
    };
    assert_eq!(answers[0].value, "jk/i p");
}

#[test]
fn paste_is_owned_by_the_custom_answer_editor() {
    let mut query = Query::new(vec![QueryQuestion {
        id: "custom".into(),
        header: "Choice".into(),
        prompt: "What should happen?".into(),
        choices: Vec::new(),
        custom_answer: QueryCustomAnswer::Allowed,
    }])
    .unwrap();

    assert_eq!(query.activate(0), Some(QueryOutcome::Consumed));
    query.handle_paste("first\r\nsecond".into());

    assert_eq!(query.view().custom_answer, Some("first second"));
}

#[test]
fn pointer_activation_answers_its_exact_target() {
    let mut query = Query::new(vec![QueryQuestion {
        id: "choice".into(),
        header: "Choose".into(),
        prompt: "Which one?".into(),
        choices: vec![
            QueryChoice {
                label: "First".into(),
                description: "First choice".into(),
            },
            QueryChoice {
                label: "Second".into(),
                description: "Second choice".into(),
            },
        ],
        custom_answer: QueryCustomAnswer::Unavailable,
    }])
    .unwrap();

    let Some(QueryOutcome::Completed(answers)) = query.activate(1) else {
        panic!("expected the pointer target to complete the query");
    };
    assert_eq!(answers[0].value, "Second");
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
