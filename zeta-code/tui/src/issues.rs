mod assignment;
mod assignment_request;
mod board;
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
use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use crossterm::event::KeyEventKind;
use crossterm::event::KeyModifiers;
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::widgets::Paragraph;
use ratatui::widgets::Wrap;
use std::collections::BTreeSet;
use std::time::Duration;
use std::time::Instant;
use zeta_protocol::CommandId;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum IssueState {
    Open,
    Closed,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Repository {
    pub(crate) host: String,
    pub(crate) owner: String,
    pub(crate) name: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Issue {
    pub(crate) labels: Vec<String>,
    pub(crate) assignees: Vec<String>,
    pub(crate) number: u64,
    pub(crate) title: String,
}

pub(crate) struct Page {
    repository: Repository,
    issues: Vec<Issue>,
    next: Option<u32>,
    refresh_after_seconds: Option<u32>,
    freshness: String,
    fetched_at: u64,
    notice: String,
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
pub(crate) enum ListMode {
    Cached,
    Auto,
    Refresh,
    ClearCache,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum Command {
    Overview {
        generation: u64,
        workflow: bool,
    },
    Assignment {
        generation: u64,
        request: assignment::Request,
    },
    OpenWork {
        session_id: zeta_protocol::SessionId,
    },
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
        query: String,
        mode: ListMode,
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

impl Command {
    pub(crate) fn is_control(&self) -> bool {
        matches!(
            self,
            Self::Assignment {
                request: assignment::Request::Batch { .. },
                ..
            }
        ) || matches!(
            self,
            Self::Assignment {
                request: assignment::Request::Act {
                    action: assignment::Action::Release | assignment::Action::Transfer(_),
                    ..
                },
                ..
            }
        )
    }
}

pub(crate) enum Event {
    Overview {
        generation: u64,
        views: Result<Vec<assignment::View>, String>,
        labels: Option<Result<github::IssueLabels, String>>,
    },
    Assignment {
        generation: u64,
        result: Result<assignment::Reply, String>,
    },
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
    assignment: assignment::Panel,
    board: board::Board,
    read_state: IssueState,
    overview_loading: bool,
    overview_at: Instant,
    workflow_loaded: bool,
    overview_error: String,
    detail_issue: Option<u64>,
    open: bool,
    busy: bool,
    pending_creation: bool,
    generation: u64,
    repository: Option<Repository>,
    selected: BTreeSet<u64>,
    cursor: usize,
    search: SearchBoxState,
    query: String,
    search_origin: Option<board::Target>,
    refresh_at: Option<Instant>,
    auto_refresh: bool,
    freshness: String,
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
            assignment: assignment::Panel::default(),
            board: board::Board::default(),
            read_state: IssueState::Open,
            overview_loading: false,
            overview_at: Instant::now(),
            workflow_loaded: false,
            overview_error: String::new(),
            detail_issue: None,
            open: false,
            busy: false,
            pending_creation: false,
            generation: 0,
            repository: None,
            selected: BTreeSet::new(),
            cursor: 0,
            search: SearchBoxState::new(SearchBoxModel::new("Search keywords or #number")),
            query: String::new(),
            search_origin: None,
            refresh_at: None,
            auto_refresh: false,
            freshness: String::new(),
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
    pub(crate) fn open_workflow(&mut self) -> Command {
        self.pr_session = None;
        self.pr = None;
        self.detail = None;
        self.detail_issue = None;
        self.open_assignment(assignment::Request::Workflow)
    }

    fn open_assignment(&mut self, request: assignment::Request) -> Command {
        self.open = true;
        self.assignment.open(&request);
        self.assignment_command(request)
    }

    fn assignment_command(&mut self, request: assignment::Request) -> Command {
        let request = if let assignment::Request::Batch { actions } = &request {
            if let Some(assignment::Request::Act { id, action, .. }) = actions.first() {
                if let Some(selected) = self
                    .board
                    .views
                    .iter()
                    .find(|view| &view.assignment.id == id)
                {
                    assignment::Request::Batch {
                        actions: self
                            .board
                            .views
                            .iter()
                            .filter(|view| {
                                view.assignment.batch_id == selected.assignment.batch_id
                                    && view.assignment.ownership
                                        == github::IssueOwnership::Held
                            })
                            .map(|view| assignment::Request::Act {
                                command_id: new_command_id("issue-batch-control"),
                                id: view.assignment.id.clone(),
                                revision: view.assignment.revision,
                                epoch: view.assignment.epoch,
                                action: action.clone(),
                            })
                            .collect(),
                    }
                } else {
                    request
                }
            } else {
                request
            }
        } else {
            request
        };
        self.generation = self.generation.wrapping_add(1);
        self.overview_loading = false;
        Command::Assignment {
            generation: self.generation,
            request,
        }
    }

    pub(crate) fn is_open(&self) -> bool {
        self.open
    }

    pub(crate) fn open(&mut self) -> Option<Command> {
        self.open = true;
        if self.pending_creation {
            return None;
        }
        self.assignment.close();
        self.read_state = IssueState::Open;
        self.detail_issue = None;
        self.overview_loading = false;
        self.overview_at = Instant::now();
        self.workflow_loaded = false;
        self.selected.clear();
        self.search.set_query(String::new());
        self.query.clear();
        self.search.set_input_active(false);
        self.attempt = None;
        self.detail = None;
        self.pr = None;
        self.pr_session = None;
        self.board.clear_pages();
        self.board.collapse(board::Group::Closed);
        self.board.selected = None;
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
        self.load_mode(page, ListMode::Cached)
    }

    fn load_mode(&mut self, page: u32, mode: ListMode) -> Command {
        self.refresh_at = None;
        self.overview_loading = false;
        self.generation = self.generation.wrapping_add(1);
        self.busy = true;
        self.status = "Loading issues...".into();
        Command::List {
            state: self.state(),
            generation: self.generation,
            page,
            query: self.query.clone(),
            mode,
        }
    }

    pub(crate) fn poll_refresh(&mut self, now: Instant) -> Option<Command> {
        if self.open && self.assignment.is_open() {
            if self.busy {
                return None;
            }
            return self
                .assignment
                .poll(now)
                .map(|request| self.assignment_command(request));
        }
        if self.open
            && !self.assignment.is_open()
            && !self.busy
            && self.detail.is_none()
            && self.pr_session.is_none()
            && !self.search.input_active()
            && self.refresh_at.is_some_and(|deadline| now >= deadline)
        {
            self.read_state = if self.board.selected_state() == IssueState::Closed
                && self.board.collapsed(board::Group::Closed)
                && !self.board.closed_loaded
            {
                IssueState::Open
            } else {
                self.board.selected_state()
            };
            return Some(self.load_mode(1, ListMode::Auto));
        }
        if self.open
            && !self.busy
            && self.detail.is_none()
            && self.pr_session.is_none()
            && !self.search.input_active()
            && !self.overview_loading
            && now >= self.overview_at
        {
            self.overview_loading = true;
            self.overview_at = now + Duration::from_secs(2);
            return Some(Command::Overview {
                generation: self.generation,
                workflow: !self.workflow_loaded,
            });
        }
        None
    }

    fn state(&self) -> IssueState {
        self.read_state
    }
    fn local_filtering(&self) -> bool {
        self.search.input_active() && self.search.query() != self.query
    }

    pub(crate) fn key_hints(&self) -> &str {
        if self.assignment.is_open() {
            return self.assignment.hints();
        }
        if self.search.input_active() {
            "Type keywords/#number · Enter search · Esc cancel"
        } else if self.detail.is_some() {
            "↑↓ scroll · Esc back"
        } else if self.pr_session.is_some() {
            "↑↓ select · Enter create · i preview · r refresh · Esc back"
        } else {
            "↑↓ select · Tab groups · Enter details/fold · Space select · d assign · ? help"
        }
    }

    pub(crate) fn handle_paste(&mut self, pasted: String) {
        if self.assignment.is_open() {
            self.assignment.paste(pasted);
            return;
        }
        if self.search.input_active() {
            self.search.handle_paste(pasted);
            self.board.reconcile(self.search.query(), true);
        }
    }

    pub(crate) fn handle_key(&mut self, key: KeyEvent) -> Option<Command> {
        if key.kind == KeyEventKind::Release {
            return None;
        }
        if key.kind == KeyEventKind::Repeat
            && !self.search.input_active()
            && !(self.assignment.is_open() && self.assignment.editing_text())
            && Navigation::from_key(key).is_none()
        {
            return None;
        }
        if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
            self.assignment.close();
            self.detail = None;
            self.detail_issue = None;
            self.open = false;
            self.busy = false;
            self.overview_loading = false;
            self.generation = self.generation.wrapping_add(1);
            return None;
        }
        if key.code == KeyCode::Char('u')
            && key.modifiers.contains(KeyModifiers::CONTROL)
            && !self.search.input_active()
            && !(self.assignment.is_open() && self.assignment.editing_text())
        {
            return None;
        }
        if key
            .modifiers
            .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT)
            && !matches!(key.code, KeyCode::Enter | KeyCode::Char('r' | 'u'))
        {
            return None;
        }
        if self.assignment.is_open() {
            return match self.assignment.handle_key(key) {
                assignment::Outcome::None => None,
                assignment::Outcome::Close => {
                    self.assignment.close();
                    self.generation = self.generation.wrapping_add(1);
                    self.busy = false;
                    self.overview_loading = false;
                    self.detail_issue = None;
                    self.overview_at = Instant::now();
                    if self.repository.is_none() {
                        Some(self.load(1))
                    } else {
                        None
                    }
                }
                assignment::Outcome::Request(request) => Some(self.assignment_command(request)),
                assignment::Outcome::OpenWork(session_id) => {
                    self.assignment.close();
                    self.open = false;
                    Some(Command::OpenWork { session_id })
                }
            };
        }
        if self.search.input_active() {
            match key.code {
                KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    self.search.set_query(String::new());
                    self.cursor = 0;
                }
                KeyCode::Esc => {
                    self.search.set_query(self.query.clone());
                    self.search.set_input_active(false);
                    self.board.selected = self.search_origin.take();
                }
                KeyCode::Enter => {
                    self.search.set_input_active(false);
                    let query = self.search.query().trim().to_owned();
                    if query != self.query {
                        self.query = query;
                        self.search.set_query(self.query.clone());
                        self.selected.clear();
                        self.attempt = None;
                        self.board.clear_pages();
                        self.board.selected = None;
                        self.search_origin = None;
                        self.cursor = 0;
                        return Some(self.load(1));
                    }
                }
                _ => {
                    if self.search.handle_key(key)
                        == search_box::SearchBoxInputOutcome::QueryChanged
                    {
                        self.cursor = 0;
                    }
                }
            }
            self.board
                .reconcile(self.search.query(), self.local_filtering());
            return None;
        }
        if key.code == KeyCode::Esc || (key.code == KeyCode::Left && self.detail.is_some()) {
            if self.detail.take().is_none() {
                self.open = false;
            }
            return None;
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
        let query = self.search.query().to_owned();
        if let Some(navigation) = Navigation::from_key(key) {
            self.board
                .navigate(navigation, &query, self.search.input_active());
            return None;
        }
        if let Some(board::Target::Group(group)) = self.board.selected.clone() {
            match key.code {
                KeyCode::Enter | KeyCode::Char(' ') => self.board.toggle(group),
                KeyCode::Right => self.board.expand(group),
                KeyCode::Left => self.board.collapse(group),
                _ => {}
            }
            if matches!(
                key.code,
                KeyCode::Enter | KeyCode::Char(' ') | KeyCode::Right | KeyCode::Left
            ) {
                if group == board::Group::Closed
                    && !self.board.collapsed(group)
                    && !self.board.closed_loaded
                {
                    self.read_state = IssueState::Closed;
                    return Some(self.load(1));
                }
                return None;
            }
        }
        let entry = self.board.selected_entry(&query, false);
        let number = entry.as_ref().map(|entry| entry.issue.number);
        match key.code {
            KeyCode::Left => { self.open = false; },
            KeyCode::Char('?') => self.detail = Some("Enter / Right: details or expand group\nLeft: collapse group or return to conversation\nSpace: select Issue or toggle group\nd: distribute selected Issues · g: combine · b: create branch\nw: workflow settings · /: search · n: next page · r: refresh\nCtrl+R: clear list cache\nManaged Issue: s resume, p pause, t transfer, u release, c cancel, v verify, y retry\nDelivery and batch actions are available in the work details.\no / Ctrl+Enter: conversation from HEAD · m / Alt+Enter: from main".into()),
            KeyCode::Char('o') => return self.start(StartPoint::CurrentBranch),
            KeyCode::Char('m') => return self.start(StartPoint::Main),
            KeyCode::Tab => self.board.jump_group(1, &query),
            KeyCode::BackTab => self.board.jump_group(-1, &query),
            KeyCode::Enter if key.modifiers.contains(KeyModifiers::CONTROL) => return self.start(StartPoint::CurrentBranch),
            KeyCode::Enter if key.modifiers.contains(KeyModifiers::ALT) => return self.start(StartPoint::Main),
            KeyCode::Char('w') => return Some(self.open_workflow()),
            KeyCode::Char('b' | 'd' | 'g') => {
                let numbers = if self.selected.is_empty() { number.into_iter().collect() } else { self.selected.iter().copied().collect() };
                let mode = if key.code == KeyCode::Char('b') { assignment::PlanMode::Branch } else if key.code == KeyCode::Char('g') { assignment::PlanMode::Combined } else { assignment::PlanMode::Distributed };
                self.detail_issue = None;
                return Some(self.open_assignment(assignment::Request::Plan { numbers, mode }));
            }
            KeyCode::Char('/') => { self.read_state = self.board.selected_state(); self.search_origin = self.board.selected.clone(); self.search.set_input_active(true); },
            KeyCode::Char('r') => {
                self.workflow_loaded = false;
                self.read_state = self.board.selected_state();
                let mode = if key.modifiers.contains(KeyModifiers::CONTROL) { ListMode::ClearCache } else { ListMode::Refresh };
                return Some(self.load_mode(1, mode));
            }
            KeyCode::Char('n') => { self.read_state = self.board.selected_state(); if let Some(page) = self.board.next(self.read_state) { return Some(self.load(page)); } }
            KeyCode::Char(' ') => { if let Some(number) = number { if !self.selected.remove(&number) { self.selected.insert(number); } self.attempt = None; } }
            KeyCode::Enter | KeyCode::Right | KeyCode::Char('i') => {
                if let Some(view) = entry.as_ref().and_then(|entry| entry.work.as_ref()) {
                    self.detail_issue = number;
                    self.assignment.show_work(view.clone());
                    if let (Some(number), Some(repository)) = (number, self.repository.clone()) {
                        self.generation = self.generation.wrapping_add(1); self.overview_loading = false; self.busy = true;
                        return Some(Command::Read { generation: self.generation, repository, number });
                    }
                } else if let (Some(number), Some(repository)) = (number, self.repository.clone()) {
                    self.generation = self.generation.wrapping_add(1); self.overview_loading = false; self.busy = true;
                    self.status = "Reading issue and comments...".into();
                    return Some(Command::Read { generation: self.generation, repository, number });
                } else if !self.status.is_empty() { self.detail = Some(self.status.clone()); }
            }
            KeyCode::Char('p' | 's' | 't' | 'u' | 'c' | 'y' | 'v' | 'P' | 'C') => {
                if let Some(view) = entry.and_then(|entry| entry.active_work().cloned()) {
                    self.detail_issue = number; self.assignment.show_work(view);
                    if let assignment::Outcome::Request(request) = self.assignment.handle_key(key) { return Some(self.assignment_command(request)); }
                }
            }
            _ => {},
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
            Event::Read { generation, result }
                if self.detail_issue.is_some() && self.assignment.is_open() =>
            {
                if generation == self.generation {
                    self.busy = false;
                    self.assignment
                        .set_material(result.unwrap_or_else(|error| {
                            format!("Issue contents unavailable: {error}")
                        }));
                }
                return;
            }
            Event::Overview {
                generation,
                views,
                labels,
            } => {
                if generation != self.generation || !self.open {
                    return;
                }
                self.overview_loading = false;
                self.overview_error.clear();
                if let Some(labels) = labels {
                    match labels {
                        Ok(labels) => {
                            self.board.labels = Some(labels);
                            self.workflow_loaded = true;
                        }
                        Err(error) => {
                            self.overview_error = error;
                            self.overview_at = Instant::now() + Duration::from_secs(60);
                        }
                    }
                }
                match views {
                    Ok(views) => self.board.views = views,
                    Err(error) => {
                        self.overview_error = error;
                        self.overview_at = Instant::now() + Duration::from_secs(60);
                    }
                }
                self.board
                    .reconcile(self.search.query(), self.local_filtering());
                return;
            }
            Event::Assignment { generation, result } => {
                if generation != self.generation || !self.open {
                    return;
                }
                self.busy = false;
                match result {
                    Ok(assignment::Reply::Assignments(views)) => {
                        for view in &views {
                            self.board
                                .views
                                .retain(|old| old.assignment.id != view.assignment.id);
                        }
                        self.board.views.splice(0..0, views.clone());
                        self.board.reconcile(self.search.query(), false);
                        if let Some(number) = self.detail_issue {
                            let views = views
                                .into_iter()
                                .filter(|view| {
                                    view.assignment
                                        .item
                                        .issues
                                        .iter()
                                        .any(|issue| issue.number == number)
                                })
                                .take(1)
                                .collect();
                            self.assignment
                                .update(Ok(assignment::Reply::Assignments(views)));
                        } else {
                            self.assignment.close();
                            self.overview_at = Instant::now();
                        }
                    }
                    Ok(assignment::Reply::Workflow(workflow)) => {
                        self.board.labels = Some(workflow.settings.labels.clone());
                        self.workflow_loaded = true;
                        self.assignment
                            .update(Ok(assignment::Reply::Workflow(workflow)));
                    }
                    result => self.assignment.update(result),
                }
                return;
            }
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
            Event::Overview { .. }
            | Event::Assignment { .. }
            | Event::ContextReceived { .. }
            | Event::PrPreviewed { .. }
            | Event::PrCreated { .. } => {
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
            Err(error) => {
                self.status = error;
                self.refresh_at = self
                    .auto_refresh
                    .then(|| Instant::now() + Duration::from_secs(60));
            }
            Ok((page, detail)) => {
                if let Some((page, result)) = page {
                    if self.repository.as_ref() != Some(&result.repository) {
                        self.selected.clear();
                        self.attempt = None;
                    }
                    if self
                        .repository
                        .as_ref()
                        .is_some_and(|repository| repository != &result.repository)
                    {
                        self.board = board::Board::default();
                        self.workflow_loaded = false;
                    }
                    self.repository = Some(result.repository);
                    self.board.page(
                        self.read_state,
                        page,
                        result.issues,
                        result.next,
                        result.fetched_at,
                    );
                    self.board
                        .reconcile(self.search.query(), self.local_filtering());
                    self.auto_refresh = result.refresh_after_seconds.is_some();
                    self.refresh_at = result.refresh_after_seconds.map(|seconds| {
                        Instant::now() + Duration::from_secs(u64::from(seconds.max(1)))
                    });
                    self.freshness = result.freshness;
                    self.status = result.notice;
                }
                if detail.is_some() {
                    self.status.clear();
                }
                self.detail = detail;
                self.detail_scroll = 0;
                if self.board.loaded() == 0 && self.status.is_empty() {
                    self.status = format!(
                        "No {} issues",
                        if self.state() == IssueState::Closed {
                            "closed"
                        } else {
                            "open"
                        }
                    );
                }
            }
        }
    }

    pub(crate) fn draw(&self, frame: &mut Frame<'_>, area: Rect, context: RenderContext<'_>) {
        if self.assignment.is_open() {
            self.assignment.draw(frame, area, context);
            return;
        }
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
        let layout = PanelLayout::new(area, 0);
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
        let summary = format!(
            "{} · {} selected · {} loaded · {}",
            self.repository
                .as_ref()
                .map(|repo| format!("{}/{}", repo.owner, repo.name))
                .unwrap_or_default(),
            self.selected.len(),
            self.board.loaded(),
            self.freshness
        );
        let extra = self
            .board
            .entries(self.search.query(), self.local_filtering())
            .len()
            .saturating_sub(self.board.loaded());
        let summary = if extra > 0 {
            format!("{summary} · {extra} from work")
        } else {
            summary
        };
        let mut lines = vec![Line::styled(summary, muted)];
        if !self.status.is_empty() {
            lines.push(Line::styled(self.status.as_str(), muted));
        }
        if !self.overview_error.is_empty() {
            lines.push(Line::styled(
                format!("Work status unavailable: {}", self.overview_error),
                muted,
            ));
        }
        let summary_height = lines.len() as u16;
        frame.render_widget(
            Paragraph::new(lines),
            Rect {
                y: area.y.saturating_add(search_height),
                height: summary_height,
                ..area
            },
        );
        let offset = search_height.saturating_add(summary_height);
        self.board.draw(
            frame,
            Rect {
                y: area.y.saturating_add(offset),
                height: area.height.saturating_sub(offset),
                ..area
            },
            self.search.query(),
            self.local_filtering(),
            &self.selected,
            context,
        );
    }
}

#[cfg(test)]
#[path = "issues_tests.rs"]
mod tests;
