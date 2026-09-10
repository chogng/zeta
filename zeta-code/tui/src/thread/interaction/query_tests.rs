use super::Query;
use super::QueryChoice;
use super::QueryCustomAnswer;
use super::QueryOutcome;
use super::QueryQuestion;
use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use crossterm::event::KeyModifiers;
use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::layout::Rect;

#[test]
fn fixed_answers_advance_pages_and_complete_once() {
    let mut query = Query::new(vec![question("one"), question("two")]).unwrap();

    assert_eq!(
        query.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)),
        QueryOutcome::Consumed
    );
    let outcome = query.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

    let QueryOutcome::Completed(answers) = outcome else {
        panic!("expected completed query");
    };
    assert_eq!(answers[0].question_id, "one");
    assert_eq!(answers[1].question_id, "two");
    assert_eq!(answers[0].value, "Yes");
    assert_eq!(
        query.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)),
        QueryOutcome::Consumed
    );
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

    query.handle_key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
    assert_eq!(
        query.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)),
        QueryOutcome::Consumed
    );
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

    assert_eq!(
        query.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)),
        QueryOutcome::Consumed
    );
    query.handle_paste("first\r\nsecond".into());

    assert_eq!(query.view().custom_answer, Some("first second"));
}

#[test]
fn keyboard_activation_answers_the_selected_choice() {
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

    query.handle_key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
    let QueryOutcome::Completed(answers) =
        query.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE))
    else {
        panic!("expected the selected choice to complete the query");
    };
    assert_eq!(answers[0].value, "Second");
}

#[test]
fn pointer_choice_uses_the_full_row_and_the_ordinary_activation_path() {
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
    let area = Rect::new(0, 0, 50, super::desired_height(query.view()));
    let second_row = area.y + 3;
    assert_eq!(
        super::choice_at(
            area,
            query.view(),
            ratatui::layout::Position::new(area.right() - 2, second_row)
        ),
        Some(1)
    );
    let mut terminal = Terminal::new(TestBackend::new(area.width, area.height)).unwrap();
    terminal
        .draw(|frame| {
            super::draw(
                frame,
                area,
                query.view(),
                Some(1),
                None,
                crate::render::test_context(),
            )
        })
        .unwrap();
    for column in area.x + 1..area.right() - 1 {
        assert_eq!(
            terminal.backend().buffer()[(column, second_row)].bg,
            crate::render::test_context().hover_background()
        );
    }
    let QueryOutcome::Completed(answers) = query.activate(1) else {
        panic!("clicking a fixed answer must use its ordinary activation path")
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
