mod request;
pub(crate) use request::execute;
pub(crate) use request::start;

use crate::client::new_command_id;
use crate::render::RenderContext;
use crate::widgets::list_selection::item_style;
use crate::widgets::navigation::Navigation;
use crate::widgets::panel;
use crate::widgets::panel::PanelLayout;
use crate::widgets::search_box;
use crate::widgets::search_box::SearchBoxModel;
use crate::widgets::search_box::SearchBoxState;
use crate::widgets::tab_list;
use crate::widgets::tab_list::FocusedTabListInputOutcome;
use crate::widgets::tab_list::TabListItem;
use crate::widgets::tab_list::TabListState;
use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use crossterm::event::KeyEventKind;
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::widgets::Paragraph;
use ratatui::widgets::Wrap;
use std::collections::BTreeSet;
use zeta_protocol::CommandId;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum IssueState {
    Open,
    Closed,
}

impl TabListItem for IssueState {
    fn tab_label(&self) -> &str {
        match self {
            Self::Open => "Open",
            Self::Closed => "Closed",
        }
    }
}

#[derive(Debug)]
struct Tabs {
    state: TabListState<IssueState>,
    focused: bool,
}

impl Default for Tabs {
    fn default() -> Self {
        Self {
            state: TabListState::new(vec![IssueState::Open, IssueState::Closed]),
            focused: false,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Repository {
    pub(crate) host: String,
    pub(crate) owner: String,
    pub(crate) name: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Issue {
    pub(crate) number: u64,
    pub(crate) title: String,
}

pub(crate) struct Page {
    repository: Repository,
    issues: Vec<Issue>,
    next: Option<u32>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum StartPoint {
    CurrentBranch,
    Main,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum PrMode {
    Ordinary,
    Draft,
    Merge,
    Squash,
    Rebase,
}

impl PrMode {
    fn label(self) -> &'static str {
        match self {
            Self::Ordinary => "Create PR",
            Self::Draft => "Create draft PR",
            Self::Merge => "Create PR; automatically merge after required checks",
            Self::Squash => "Create PR; automatically squash after required checks",
            Self::Rebase => "Create PR; automatically rebase after required checks",
        }
    }
}

#[derive(Debug)]
pub(crate) struct PrPlan {
    summary: String,
    expected_tree: String,
    details: String,
    modes: Vec<PrMode>,
    existing: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum Command {
    PreviewPr {
        generation: u64,
        session_id: zeta_protocol::SessionId,
    },
    CreatePr {
        generation: u64,
        session_id: zeta_protocol::SessionId,
        command_id: CommandId,
        expected_tree: String,
        mode: PrMode,
    },
    List {
        state: IssueState,
        generation: u64,
        page: u32,
    },
    Read {
        generation: u64,
        repository: Repository,
        number: u64,
    },
    Start {
        generation: u64,
        command_id: CommandId,
        repository: Repository,
        numbers: Vec<u64>,
        start: StartPoint,
    },
}

pub(crate) enum Event {
    PrPreviewed {
        generation: u64,
        result: Result<PrPlan, String>,
    },
    PrCreated {
        generation: u64,
        result: Result<String, String>,
    },
    ContextReceived {
        session_id: zeta_protocol::SessionId,
        numbers: Vec<u64>,
    },
    Listed {
        generation: u64,
        page: u32,
        result: Result<Page, String>,
    },
    Read {
        generation: u64,
        result: Result<String, String>,
    },
}

#[derive(Debug)]
pub(crate) struct Manager {
    tabs: Tabs,
    open: bool,
    busy: bool,
    pending_creation: bool,
    generation: u64,
    repository: Option<Repository>,
    issues: Vec<Issue>,
    next: Option<u32>,
    selected: BTreeSet<u64>,
    cursor: usize,
    search: SearchBoxState,
    detail: Option<String>,
    detail_scroll: usize,
    status: String,
    attempt: Option<(Vec<u64>, StartPoint, CommandId)>,
    pr: Option<PrPlan>,
    pr_session: Option<zeta_protocol::SessionId>,
}

impl Default for Manager {
    fn default() -> Self {
        Self {
            tabs: Tabs::default(),
            open: false,
            busy: false,
            pending_creation: false,
            generation: 0,
            repository: None,
            issues: Vec::new(),
            next: None,
            selected: BTreeSet::new(),
            cursor: 0,
            search: SearchBoxState::new(SearchBoxModel::new("Filter loaded issues")),
            detail: None,
            detail_scroll: 0,
            status: String::new(),
            attempt: None,
            pr: None,
            pr_session: None,
        }
    }
}

impl Manager {
    pub(crate) fn is_open(&self) -> bool {
        self.open
    }

    pub(crate) fn open(&mut self) -> Option<Command> {
        self.open = true;
        if self.pending_creation {
            return None;
        }
        self.tabs = Tabs::default();
        self.selected.clear();
        self.search.set_query(String::new());
        self.search.set_input_active(false);
        self.attempt = None;
        self.detail = None;
        self.pr = None;
        self.pr_session = None;
        Some(self.load(1))
    }

    pub(crate) fn open_pr(&mut self, session_id: zeta_protocol::SessionId) -> Option<Command> {
        self.open = true;
        if self.pending_creation {
            return None;
        }
        self.pr_session = Some(session_id.clone());
        self.pr = None;
        self.detail = None;
        self.cursor = 0;
        self.generation = self.generation.wrapping_add(1);
        self.busy = true;
        self.status = "Reading PR preview...".into();
        Some(Command::PreviewPr {
            generation: self.generation,
            session_id,
        })
    }

    fn load(&mut self, page: u32) -> Command {
        if page == 1 {
            self.issues.clear();
            self.next = None;
            self.cursor = 0;
        }
        self.generation = self.generation.wrapping_add(1);
        self.busy = true;
        self.status = "Loading issues...".into();
        Command::List {
            state: self.state(),
            generation: self.generation,
            page,
        }
    }

    fn state(&self) -> IssueState {
        *self
            .tabs
            .state
            .active_tab()
            .expect("issue tabs are enabled")
    }

    fn switch_tab(&mut self) -> Command {
        self.issues.clear();
        self.selected.clear();
        self.attempt = None;
        self.next = None;
        self.cursor = 0;
        self.search.set_query(String::new());
        self.load(1)
    }

    pub(crate) fn key_hints(&self) -> &str {
        if self.search.input_active() {
            "Type to filter · Enter done · Esc back"
        } else if self.detail.is_some() {
            "↑↓ scroll · Esc back"
        } else if self.pr_session.is_some() {
            "↑↓ select · Enter create · i preview · r refresh · Esc back"
        } else if self.tabs.focused {
            "←→/Tab switch · ↓/Enter list · Esc close"
        } else {
            "Space select · Enter open/start · Tab focus · / search · n more · r refresh · Esc close"
        }
    }

    fn filtered(&self) -> Vec<&Issue> {
        let query = self.search.query().to_lowercase();
        self.issues
            .iter()
            .filter(|issue| {
                format!("#{} {}", issue.number, issue.title)
                    .to_lowercase()
                    .contains(&query)
            })
            .collect()
    }

    pub(crate) fn handle_key(&mut self, key: KeyEvent) -> Option<Command> {
        if key.kind == KeyEventKind::Release {
            return None;
        }
        if self.search.input_active() {
            match key.code {
                KeyCode::Esc | KeyCode::Enter => self.search.set_input_active(false),
                _ => {
                    if self.search.handle_key(key)
                        == search_box::SearchBoxInputOutcome::QueryChanged
                    {
                        self.cursor = 0;
                    }
                }
            }
            return None;
        }
        if key.code == KeyCode::Esc || (key.code == KeyCode::Left && self.detail.is_some()) {
            if self.detail.take().is_none() {
                self.open = false;
            }
            return None;
        }
        if self.pr_session.is_none() && self.detail.is_none() && !self.pending_creation {
            if self.tabs.focused {
                match self.tabs.state.handle_focused_key(key) {
                    FocusedTabListInputOutcome::ActiveChanged => return Some(self.switch_tab()),
                    FocusedTabListInputOutcome::EnterContent
                    | FocusedTabListInputOutcome::FocusNext => {
                        self.tabs.focused = false;
                        self.cursor = 0;
                    }
                    _ => {}
                }
                return None;
            }
            if key.code == KeyCode::BackTab
                && (self.cursor == 0 || self.cursor < self.filtered().len())
                || key.code == KeyCode::Up && self.cursor == 0
            {
                self.tabs.focused = true;
                return None;
            }
        }
        if self.busy {
            return None;
        }
        if let Some(detail) = &self.detail {
            if let Some(navigation) = Navigation::from_key(key) {
                self.detail_scroll =
                    navigation.offset(self.detail_scroll, detail.chars().count(), 12);
            }
            return None;
        }
        if let Some(session_id) = self.pr_session.clone() {
            if key.code == KeyCode::Char('r') {
                return self.open_pr(session_id);
            }
            if let Some(pr) = &self.pr {
                if let Some(navigation) = Navigation::from_key(key) {
                    self.cursor =
                        navigation.offset(self.cursor, pr.modes.len().saturating_sub(1), 5);
                } else if key.code == KeyCode::Char('i') {
                    self.detail = Some(pr.details.clone());
                } else if key.code == KeyCode::Enter {
                    let mode = *pr.modes.get(self.cursor)?;
                    self.generation = self.generation.wrapping_add(1);
                    self.busy = true;
                    self.status =
                        "Committing task changes, pushing branch and creating PR...".into();
                    return Some(Command::CreatePr {
                        generation: self.generation,
                        session_id,
                        command_id: new_command_id("issue-pr"),
                        expected_tree: pr.expected_tree.clone(),
                        mode,
                    });
                }
            }
            return None;
        }
        let rows = self.filtered();
        let count = rows.len();
        if let Some(navigation) = Navigation::from_key(key) {
            self.cursor = navigation.offset(self.cursor, count + 1, 12);
            return None;
        }
        let number = rows.get(self.cursor).map(|issue| issue.number);
        match key.code {
            KeyCode::BackTab => {
                self.cursor = if self.cursor > count {
                    count
                } else if self.cursor == count {
                    0
                } else {
                    count + 1
                };
            }
            KeyCode::Tab => {
                self.cursor = if self.cursor < count {
                    count
                } else if self.cursor == count {
                    count + 1
                } else {
                    self.tabs.focused = true;
                    0
                }
            }
            KeyCode::Char('/') => self.search.set_input_active(true),
            KeyCode::Char('i') if !self.status.is_empty() => {
                self.detail = Some(self.status.clone())
            }
            KeyCode::Char('r') => return Some(self.load(1)),
            KeyCode::Char('n') => {
                if let Some(page) = self.next {
                    return Some(self.load(page));
                }
            }
            KeyCode::Char(' ') => {
                if let Some(number) = number {
                    if !self.selected.remove(&number) {
                        self.selected.insert(number);
                    }
                    self.attempt = None;
                }
            }
            KeyCode::Enter if self.cursor >= count => {
                return self.start(if self.cursor == count {
                    StartPoint::CurrentBranch
                } else {
                    StartPoint::Main
                });
            }
            KeyCode::Enter => {
                if let (Some(number), Some(repository)) = (number, self.repository.clone()) {
                    self.generation = self.generation.wrapping_add(1);
                    self.busy = true;
                    self.status = "Reading issue and comments...".into();
                    return Some(Command::Read {
                        generation: self.generation,
                        repository,
                        number,
                    });
                }
            }
            _ => {}
        }
        None
    }

    fn start(&mut self, start: StartPoint) -> Option<Command> {
        if self.selected.is_empty() {
            self.status = "Select at least one issue with Space".into();
            return None;
        }
        let repository = self.repository.clone()?;
        let numbers = self.selected.iter().copied().collect::<Vec<_>>();
        let command_id = match &self.attempt {
            Some((previous, point, id)) if previous == &numbers && *point == start => id.clone(),
            _ => new_command_id("issues"),
        };
        self.attempt = Some((numbers.clone(), start, command_id.clone()));
        self.busy = true;
        self.status = "Preparing issue conversation...".into();
        self.pending_creation = true;
        self.generation = self.generation.wrapping_add(1);
        Some(Command::Start {
            generation: self.generation,
            command_id,
            repository,
            numbers,
            start,
        })
    }

    pub(crate) fn finish_start(&mut self, generation: u64, error: Option<String>) {
        if generation != self.generation {
            return;
        }
        self.busy = false;
        self.pending_creation = false;
        match error {
            Some(error) => self.status = error,
            None => {
                self.open = false;
                self.selected.clear();
                self.attempt = None;
            }
        }
    }

    pub(crate) fn update(&mut self, event: Event) {
        match event {
            Event::PrPreviewed { generation, result } => {
                if generation != self.generation {
                    return;
                }
                self.busy = false;
                match result {
                    Ok(pr) => {
                        self.status = pr.existing.clone().unwrap_or_default();
                        self.pr = Some(pr);
                    }
                    Err(error) => self.status = error,
                }
                return;
            }
            Event::PrCreated { generation, result } => {
                if generation != self.generation {
                    return;
                }
                self.busy = false;
                self.status = match result {
                    Ok(status) => status,
                    Err(error) => error,
                };
                return;
            }
            _ => {}
        }
        let (generation, result) = match event {
            Event::ContextReceived { .. } | Event::PrPreviewed { .. } | Event::PrCreated { .. } => {
                return;
            }
            Event::Listed {
                generation,
                page,
                result,
            } => (
                generation,
                result.map(|result| (Some((page, result)), None)),
            ),
            Event::Read { generation, result } => {
                (generation, result.map(|result| (None, Some(result))))
            }
        };
        if generation != self.generation {
            return;
        }
        self.busy = false;
        match result {
            Err(error) => self.status = error,
            Ok((page, detail)) => {
                if let Some((page, result)) = page {
                    if self.repository.as_ref() != Some(&result.repository) {
                        self.selected.clear();
                        self.attempt = None;
                    }
                    self.repository = Some(result.repository);
                    if page == 1 {
                        self.issues.clear();
                        self.cursor = 0;
                    }
                    for issue in result.issues {
                        if let Some(existing) = self
                            .issues
                            .iter_mut()
                            .find(|existing| existing.number == issue.number)
                        {
                            *existing = issue;
                        } else {
                            self.issues.push(issue);
                        }
                    }
                    self.next = result.next;
                }
                self.detail = detail;
                self.detail_scroll = 0;
                self.status = if self.issues.is_empty() {
                    format!("No {} issues", self.state().tab_label().to_lowercase())
                } else {
                    String::new()
                };
            }
        }
    }

    pub(crate) fn draw(&self, frame: &mut Frame<'_>, area: Rect, context: RenderContext<'_>) {
        let style = Style::default()
            .fg(context.foreground())
            .bg(context.background());
        let muted = style.fg(context.muted());
        let row_style = |selected| style.patch(item_style(context, selected, false, false));
        let is_pr = self.pr_session.is_some();
        panel::draw_header(
            frame,
            area,
            if is_pr { "Issue task PR" } else { "Issues" },
            context.focus(),
        );
        let tab_rows = if is_pr {
            0
        } else {
            tab_list::desired_height(
                self.tabs.state.tabs(),
                PanelLayout::content_width(area.width),
            )
        };
        let layout = PanelLayout::new(area, tab_rows);
        if !is_pr {
            tab_list::draw(
                frame,
                layout.tabs,
                &self.tabs.state,
                self.tabs.focused && self.detail.is_none(),
                None,
                None,
                context,
            );
        }
        let area = layout.body;
        if let Some(detail) = &self.detail {
            frame.render_widget(
                Paragraph::new(detail.as_str())
                    .style(style)
                    .wrap(Wrap { trim: false })
                    .scroll((self.detail_scroll.min(u16::MAX as usize) as u16, 0)),
                area,
            );
            return;
        }
        if is_pr {
            let mut lines = vec![Line::styled(
                "Commit task changes, push branch and create PR",
                muted,
            )];
            if let Some(pr) = &self.pr {
                lines.push(Line::from(pr.summary.as_str()));
                lines.push(Line::default());
                for (index, mode) in pr.modes.iter().enumerate() {
                    lines.push(Line::styled(
                        format!(
                            "{}{}",
                            crate::render::selection_marker(self.cursor == index),
                            mode.label()
                        ),
                        row_style(self.cursor == index),
                    ));
                }
            }
            lines.push(Line::styled(self.status.as_str(), muted));
            frame.render_widget(
                Paragraph::new(lines)
                    .style(style)
                    .wrap(Wrap { trim: false }),
                area,
            );
            return;
        }
        let rows = self.filtered();
        let summary = match &self.repository {
            Some(repository) => format!(
                "{}/{} · {} selected",
                repository.owner,
                repository.name,
                self.selected.len()
            ),
            None => format!("{} selected", self.selected.len()),
        };
        let search_height = search_box::SEARCH_BOX_HEIGHT.min(area.height.saturating_sub(3));
        search_box::draw(
            frame,
            Rect {
                height: search_height,
                ..area
            },
            &self.search,
            false,
            false,
            context,
        );
        let area = Rect {
            y: area.y.saturating_add(search_height),
            height: area.height.saturating_sub(search_height),
            ..area
        };
        let mut lines = vec![Line::styled(summary, muted)];
        if !self.status.is_empty() {
            lines.push(Line::styled(self.status.as_str(), muted));
        } else if rows.is_empty() {
            lines.push(Line::styled("No matching issues", muted));
        }
        let footer_rows = 3.min(area.height);
        let body_rows = area.height.saturating_sub(footer_rows);
        let visible = usize::from(body_rows).saturating_sub(lines.len());
        let offset = self
            .cursor
            .saturating_sub(visible.saturating_sub(1))
            .min(rows.len().saturating_sub(visible));
        for (index, issue) in rows.iter().enumerate().skip(offset).take(visible) {
            let focused = !self.tabs.focused && !self.search.input_active() && self.cursor == index;
            let marker = if self.selected.contains(&issue.number) {
                "[x]"
            } else {
                "[ ]"
            };
            lines.push(Line::styled(
                format!(
                    "{}{marker} #{} {}",
                    crate::render::selection_marker(focused),
                    issue.number,
                    issue.title.replace(['\n', '\r'], " ")
                ),
                row_style(self.cursor == index),
            ));
        }
        frame.render_widget(
            Paragraph::new(lines).style(style),
            Rect {
                height: body_rows,
                ..area
            },
        );
        let mut actions = Vec::new();
        for (index, label) in [
            "Start conversation from current branch",
            "Start conversation from main",
        ]
        .iter()
        .enumerate()
        {
            let focused = !self.tabs.focused
                && !self.search.input_active()
                && self.cursor == rows.len() + index;
            actions.push(Line::styled(
                format!("{}{label}", crate::render::selection_marker(focused)),
                row_style(focused),
            ));
        }
        actions.push(Line::styled("Uncommitted changes are excluded", muted));
        frame.render_widget(
            Paragraph::new(actions).style(style),
            Rect {
                y: area.y.saturating_add(body_rows),
                height: footer_rows,
                ..area
            },
        );
    }
}

#[cfg(test)]
#[path = "issues_tests.rs"]
mod tests;
