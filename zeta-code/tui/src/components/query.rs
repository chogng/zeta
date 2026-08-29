use crate::ui::background;
use crate::ui::highlight;
use crate::ui::muted;
use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use crossterm::event::KeyEventKind;
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::Modifier;
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::text::Span;
use ratatui::widgets::Block;
use ratatui::widgets::Borders;
use ratatui::widgets::Paragraph;

const MAX_CHOICE_ROWS: usize = 8;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct QueryChoice {
    pub(crate) label: String,
    pub(crate) description: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum QueryCustomAnswer {
    Allowed,
    Unavailable,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct QueryQuestion {
    pub(crate) id: String,
    pub(crate) header: String,
    pub(crate) prompt: String,
    pub(crate) choices: Vec<QueryChoice>,
    pub(crate) custom_answer: QueryCustomAnswer,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct QueryAnswer {
    pub(crate) question_id: String,
    pub(crate) value: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum QueryOutcome {
    BeginCustomAnswer,
    Completed(Vec<QueryAnswer>),
    Consumed,
    Unhandled,
}

#[derive(Debug)]
pub(crate) struct Query {
    questions: Vec<QueryQuestion>,
    current: usize,
    selected: usize,
    answers: Vec<QueryAnswer>,
    submitting: bool,
    error: Option<String>,
}

impl Query {
    pub(crate) fn new(questions: Vec<QueryQuestion>) -> Result<Self, String> {
        if questions.is_empty() {
            return Err("a query requires at least one question".into());
        }
        if questions.iter().any(|question| {
            question.choices.is_empty() && question.custom_answer == QueryCustomAnswer::Unavailable
        }) {
            return Err("every query question requires a choice or a custom answer".into());
        }
        Ok(Self {
            questions,
            current: 0,
            selected: 0,
            answers: Vec::new(),
            submitting: false,
            error: None,
        })
    }

    pub(crate) fn handle_key(&mut self, key: KeyEvent) -> QueryOutcome {
        if key.kind != KeyEventKind::Press || self.submitting {
            return QueryOutcome::Consumed;
        }
        let option_count = self.option_count();
        match key.code {
            KeyCode::Up => {
                self.selected = self.selected.checked_sub(1).unwrap_or(option_count - 1);
                QueryOutcome::Consumed
            }
            KeyCode::Down => {
                self.selected = (self.selected + 1) % option_count;
                QueryOutcome::Consumed
            }
            KeyCode::Enter if key.modifiers.is_empty() => self.activate_selected(),
            KeyCode::Esc => QueryOutcome::Consumed,
            _ => QueryOutcome::Unhandled,
        }
    }

    pub(crate) fn submit_custom_answer(&mut self, value: String) -> QueryOutcome {
        self.advance(value)
    }

    pub(crate) fn select(&mut self, index: usize) -> bool {
        if self.submitting || index >= self.option_count() {
            return false;
        }
        self.selected = index;
        true
    }

    pub(crate) fn activate(&mut self, index: usize) -> Option<QueryOutcome> {
        self.select(index).then(|| self.activate_selected())
    }

    pub(crate) fn submission_failed(&mut self, error: String) {
        self.submitting = false;
        self.error = Some(error);
    }

    pub(crate) fn view(&self) -> QueryView<'_> {
        QueryView {
            question: &self.questions[self.current],
            current: self.current,
            total: self.questions.len(),
            selected: self.selected,
            submitting: self.submitting,
            error: self.error.as_deref(),
        }
    }

    fn activate_selected(&mut self) -> QueryOutcome {
        let question = &self.questions[self.current];
        if self.selected < question.choices.len() {
            let value = question.choices[self.selected].label.clone();
            return self.advance(value);
        }
        QueryOutcome::BeginCustomAnswer
    }

    fn advance(&mut self, value: String) -> QueryOutcome {
        let question_id = self.questions[self.current].id.clone();
        self.answers.push(QueryAnswer { question_id, value });
        self.current += 1;
        self.selected = 0;
        self.error = None;
        if self.current == self.questions.len() {
            self.submitting = true;
            QueryOutcome::Completed(self.answers.clone())
        } else {
            QueryOutcome::Consumed
        }
    }

    fn option_count(&self) -> usize {
        let question = &self.questions[self.current];
        question.choices.len() + usize::from(question.custom_answer == QueryCustomAnswer::Allowed)
    }
}

#[derive(Clone, Copy)]
pub(crate) struct QueryView<'a> {
    pub(crate) question: &'a QueryQuestion,
    pub(crate) current: usize,
    pub(crate) total: usize,
    pub(crate) selected: usize,
    pub(crate) submitting: bool,
    pub(crate) error: Option<&'a str>,
}

pub(crate) fn desired_height(view: QueryView<'_>) -> u16 {
    let choice_count = view.question.choices.len()
        + usize::from(view.question.custom_answer == QueryCustomAnswer::Allowed);
    let content_rows = 2usize
        .saturating_add(choice_count.min(MAX_CHOICE_ROWS))
        .saturating_add(usize::from(view.submitting || view.error.is_some()));
    u16::try_from(content_rows.saturating_add(2)).unwrap_or(u16::MAX)
}

pub(crate) fn draw(frame: &mut Frame<'_>, area: Rect, view: QueryView<'_>) {
    let mut lines = vec![Line::styled(
        &view.question.prompt,
        Style::default().add_modifier(Modifier::BOLD),
    )];
    lines.extend(
        view.question
            .choices
            .iter()
            .enumerate()
            .take(MAX_CHOICE_ROWS)
            .map(|(index, choice)| {
                choice_line(&choice.label, &choice.description, index == view.selected)
            }),
    );
    if view.question.custom_answer == QueryCustomAnswer::Allowed
        && view.question.choices.len() < MAX_CHOICE_ROWS
    {
        lines.push(choice_line(
            "自己输入",
            "在下方输入框中回答",
            view.selected == view.question.choices.len(),
        ));
    }
    if view.submitting {
        lines.push(Line::styled("Submitting…", Style::default().fg(muted())));
    } else if let Some(error) = view.error {
        lines.push(Line::styled(
            error,
            Style::default().fg(ratatui::style::Color::Red),
        ));
    }
    frame.render_widget(
        Paragraph::new(lines).block(
            Block::default()
                .title(format!(
                    "{}  ({}/{})",
                    view.question.header,
                    view.current + 1,
                    view.total
                ))
                .borders(Borders::ALL)
                .style(Style::default().bg(background())),
        ),
        area,
    );
}

pub(crate) fn choice_index_at(
    area: Rect,
    view: QueryView<'_>,
    column: u16,
    row: u16,
) -> Option<usize> {
    if column <= area.x || column >= area.right().saturating_sub(1) {
        return None;
    }
    let first_choice_row = area.y.saturating_add(2);
    let visible_choices = view.question.choices.len().min(MAX_CHOICE_ROWS)
        + usize::from(
            view.question.custom_answer == QueryCustomAnswer::Allowed
                && view.question.choices.len() < MAX_CHOICE_ROWS,
        );
    let index = usize::from(row.saturating_sub(first_choice_row));
    (row >= first_choice_row && index < visible_choices).then_some(index)
}

fn choice_line<'a>(label: &'a str, description: &'a str, selected: bool) -> Line<'a> {
    let marker = if selected { "› " } else { "  " };
    let style = if selected {
        Style::default()
            .fg(highlight())
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    };
    Line::from(vec![
        Span::styled(marker, style),
        Span::styled(label, style),
        Span::styled(format!("  {description}"), Style::default().fg(muted())),
    ])
}

#[cfg(test)]
#[path = "query/state_tests.rs"]
mod tests;
