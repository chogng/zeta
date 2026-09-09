use crate::render::RenderContext;
use crate::widgets::navigation::Navigation;
use crate::widgets::panel;
use crate::widgets::panel::PanelLayout;
use crate::widgets::search_box::SearchBoxModel;
use crate::widgets::search_box::SearchBoxState;
use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use crossterm::event::KeyModifiers;
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::widgets::Paragraph;
use zeta_protocol::CommandId;
use github::IssueAssignment;
use github::IssueAssignmentPlan;
use github::IssueRepositoryIdentity;
use github::IssueStage;
use github::IssueWorkflow;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum PlanMode {
    Branch,
    Combined,
    Distributed,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum StartAction {
    CreateBranch,
    Claim,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum Action {
    Release,
    RetrySync,
    Transfer(String),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Label {
    pub(crate) name: String,
    pub(crate) color: String,
    pub(crate) id: String,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Workflow {
    pub(crate) repository: IssueRepositoryIdentity,
    pub(crate) revision: u64,
    pub(crate) settings: IssueWorkflow,
    pub(crate) default_branch: String,
    pub(crate) labels: Vec<Label>,
    pub(crate) assignees: Vec<String>,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct View {
    pub(crate) assignment: IssueAssignment,
    pub(crate) stage: IssueStage,
    pub(crate) health: String,
    pub(crate) branch_url: Option<String>,
    pub(crate) pr_url: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum Request {
    Workflow,
    SaveWorkflow {
        command_id: CommandId,
        workflow: Workflow,
    },
    CreateLabels(Workflow),
    Plan {
        numbers: Vec<u64>,
        mode: PlanMode,
    },
    Start {
        command_id: CommandId,
        plan: IssueAssignmentPlan,
        action: StartAction,
    },
    List,
    Batch {
        actions: Vec<Request>,
    },
    Act {
        command_id: CommandId,
        id: String,
        revision: u64,
        epoch: u64,
        action: Action,
    },
}

pub(crate) enum Reply {
    Workflow(Workflow),
    Plan(IssueAssignmentPlan),
    Assignments(Vec<View>),
}
#[derive(Debug)]
enum Screen {
    Closed,
    Workflow(Option<Workflow>),
    Plan(Option<IssueAssignmentPlan>),
    Assignments(Vec<View>),
}
#[derive(Debug)]
pub(crate) struct Panel {
    screen: Screen,
    cursor: usize,
    search: SearchBoxState,
    edit: Option<usize>,
    plan_item: Option<usize>,
    transfer: Option<(String, u64, u64)>,
    filter: bool,
    busy: bool,
    polling: bool,
    details: bool,
    detail_scroll: u16,
    refresh_at: std::time::Instant,
    notice: String,
    material: String,
    start_id: Option<CommandId>,
    mode: PlanMode,
}
impl Default for Panel {
    fn default() -> Self {
        Self {
            screen: Screen::Closed,
            cursor: 0,
            search: SearchBoxState::new(SearchBoxModel::new("Value")),
            edit: None,
            plan_item: None,
            transfer: None,
            filter: false,
            busy: false,
            polling: false,
            details: false,
            detail_scroll: 0,
            refresh_at: std::time::Instant::now(),
            notice: String::new(),
            material: String::new(),
            start_id: None,
            mode: PlanMode::Distributed,
        }
    }
}
pub(crate) enum Outcome {
    None,
    Close,
    Request(Request),
    OpenWork(zeta_protocol::SessionId),
}

impl Panel {
    pub(super) fn editing_text(&self) -> bool {
        self.edit.is_some() || self.filter
    }
    pub(super) fn set_material(&mut self, material: String) {
        self.material = material;
    }
    pub(super) fn show_work(&mut self, view: View) {
        self.open(&Request::List);
        self.update(Ok(Reply::Assignments(vec![view])));
    }

    pub(crate) fn is_open(&self) -> bool {
        !matches!(self.screen, Screen::Closed)
    }
    pub(crate) fn close(&mut self) {
        self.screen = Screen::Closed;
        self.busy = false;
        self.polling = false;
    }
    pub(crate) fn open(&mut self, request: &Request) {
        self.material.clear();
        self.cursor = 0;
        self.details = false;
        self.detail_scroll = 0;
        self.edit = None;
        self.plan_item = None;
        self.transfer = None;
        self.filter = false;
        self.search.set_query(String::new());
        self.search.set_input_active(false);
        self.start_id = None;
        self.screen = match request {
            Request::Workflow => Screen::Workflow(None),
            Request::Plan { mode, .. } => {
                self.mode = mode.clone();
                Screen::Plan(None)
            }
            _ => Screen::Assignments(Vec::new()),
        };
        self.begin();
    }
    fn begin(&mut self) {
        self.busy = true;
        self.polling = false;
        self.notice = "Loading...".into();
    }
    pub(crate) fn poll(&mut self, now: std::time::Instant) -> Option<Request> {
        if matches!(self.screen, Screen::Assignments(_))
            && !self.busy
            && !self.polling
            && !self.details
            && self.edit.is_none()
            && !self.filter
            && now >= self.refresh_at
        {
            self.polling = true;
            self.refresh_at = now + std::time::Duration::from_secs(2);
            return Some(Request::List);
        }
        None
    }

    pub(crate) fn update(&mut self, reply: Result<Reply, String>) {
        let background = self.polling;
        self.busy = false;
        self.polling = false;
        self.refresh_at = std::time::Instant::now() + std::time::Duration::from_secs(2);
        match reply {
            Err(error) => self.notice = error,
            Ok(reply) => {
                if !background {
                    self.notice.clear();
                }
                self.screen = match reply {
                    Reply::Workflow(workflow) => Screen::Workflow(Some(workflow)),
                    Reply::Plan(plan) => Screen::Plan(Some(plan)),
                    Reply::Assignments(assignments) => Screen::Assignments(assignments),
                };
            }
        }
        self.cursor = self.cursor.min(self.rows().len().saturating_sub(1));
    }
    pub(crate) fn paste(&mut self, text: String) {
        if matches!(self.screen, Screen::Workflow(_)) && self.edit == Some(16) {
            self.search.set_query(text.replace("\r\n", "\n"));
        } else {
            self.search.handle_paste(text);
        }
    }
    pub(crate) fn hints(&self) -> &'static str {
        if self.details {
            return "↑↓ / PgUp / PgDn scroll · Esc back";
        }
        if self.edit.is_some() {
            "Type value · Ctrl+U clear · Enter save field · Esc cancel"
        } else if self.filter {
            "Type filter · Enter done · Esc clear"
        } else {
            match self.screen {
                Screen::Workflow(_) => {
                    "↑↓ select · Tab choose existing · Enter edit · s save · l create missing labels · Esc back"
                }
                Screen::Plan(_) => {
                    "↑↓ select · Enter edit · m merge next · x split · s start · c claim · Esc back"
                }
                _ => "Enter open · s resume · p pause · v verify · d deliver · i details · ? help",
            }
        }
    }
    pub(crate) fn handle_key(&mut self, key: KeyEvent) -> Outcome {
        if self.details {
            match key.code {
                KeyCode::Esc => self.details = false,
                KeyCode::Up => self.detail_scroll = self.detail_scroll.saturating_sub(1),
                KeyCode::Down => self.detail_scroll = self.detail_scroll.saturating_add(1),
                KeyCode::PageUp => self.detail_scroll = self.detail_scroll.saturating_sub(10),
                KeyCode::PageDown => self.detail_scroll = self.detail_scroll.saturating_add(10),
                KeyCode::Home => self.detail_scroll = 0,
                _ => {}
            }
            return Outcome::None;
        }
        if matches!(self.screen, Screen::Assignments(_))
            && !self.filter
            && self.edit.is_none()
            && matches!(key.code, KeyCode::Char('i') | KeyCode::Char('?'))
        {
            self.details = true;
            self.detail_scroll = 0;
            return Outcome::None;
        }
        if self.edit.is_some() || self.filter {
            match key.code {
                KeyCode::Esc => {
                    self.edit = None;
                    self.filter = false;
                    self.search.set_input_active(false);
                    self.search.set_query(String::new());
                }
                KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    self.search.set_query(String::new())
                }
                KeyCode::Enter => {
                    if let Some(index) = self.edit {
                        match self.set_field(index, self.search.query().to_owned()) {
                            Ok(Some(request)) => {
                                self.edit = None;
                                self.search.set_input_active(false);
                                self.begin();
                                return Outcome::Request(request);
                            }
                            Ok(None) => {
                                self.edit = None;
                                self.search.set_input_active(false);
                            }
                            Err(error) => self.notice = error,
                        }
                    } else {
                        self.filter = false;
                        self.search.set_input_active(false);
                    }
                }
                _ => {
                    self.search.handle_key(key);
                }
            }
            return Outcome::None;
        }
        if key.code == KeyCode::Esc && self.plan_item.take().is_some() {
            self.cursor = 0;
            return Outcome::None;
        }
        if key.code == KeyCode::Esc {
            self.close();
            return Outcome::Close;
        }
        if self.busy
            && !(matches!(self.screen, Screen::Assignments(_))
                && matches!(key.code, KeyCode::Char('p' | 'c' | 'u')))
        {
            return Outcome::None;
        }
        if let Some(navigation) = Navigation::from_key(key) {
            self.cursor = navigation.offset(self.cursor, self.rows().len().saturating_sub(1), 8);
            return Outcome::None;
        }
        let request = match &mut self.screen {
            Screen::Workflow(Some(workflow)) => match key.code {
                KeyCode::Tab if self.cursor <= 5 => {
                    let values = if self.cursor == 5 {
                        workflow.assignees.clone()
                    } else {
                        workflow
                            .labels
                            .iter()
                            .map(|label| label.name.clone())
                            .collect()
                    };
                    let current = workflow_fields(&workflow.settings)[self.cursor].1.clone();
                    if !values.is_empty() {
                        let next = values
                            .iter()
                            .position(|value| value == &current)
                            .map(|index| (index + 1) % values.len())
                            .unwrap_or(0);
                        if let Err(error) =
                            set_workflow_field(&mut workflow.settings, self.cursor, &values[next])
                        {
                            self.notice = error;
                        }
                    }
                    None
                }
                KeyCode::Char('s') => Some(Request::SaveWorkflow {
                    command_id: crate::client::new_command_id("issue-workflow"),
                    workflow: workflow.clone(),
                }),
                KeyCode::Char('l') => Some(Request::CreateLabels(workflow.clone())),
                KeyCode::Enter => {
                    let values = workflow_fields(&workflow.settings);
                    if let Some((_, value)) = values.get(self.cursor) {
                        self.search.set_query(value.clone());
                        self.search.set_input_active(true);
                        self.edit = Some(self.cursor);
                    }
                    None
                }
                _ => None,
            },
            Screen::Plan(Some(plan)) => match key.code {
                KeyCode::Char('s') | KeyCode::Char('c') => {
                    let id = self
                        .start_id
                        .get_or_insert_with(|| crate::client::new_command_id("issue-batch"))
                        .clone();
                    Some(Request::Start {
                        command_id: id,
                        plan: plan.clone(),
                        action: if self.mode == PlanMode::Branch {
                            StartAction::CreateBranch
                        } else {
                            StartAction::Claim
                        },
                    })
                }
                KeyCode::Enter => {
                    if let Some(index) = self.plan_item {
                        if let Some((_, value)) = plan
                            .items
                            .get(index)
                            .and_then(|item| item_fields(item).get(self.cursor).cloned())
                        {
                            self.search.set_query(value);
                            self.search.set_input_active(true);
                            self.edit = Some(self.cursor);
                        }
                    } else if self.cursor < plan.items.len() {
                        self.plan_item = Some(self.cursor);
                        self.cursor = 0;
                    }
                    None
                }
                KeyCode::Char('m')
                    if self.plan_item.is_none() && self.cursor + 1 < plan.items.len() =>
                {
                    let original = plan.clone();
                    let next = plan.items.remove(self.cursor + 1);
                    let id = plan.items[self.cursor].id.clone();
                    let item = &mut plan.items[self.cursor];
                    item.scope.paths.extend(next.scope.paths);
                    item.scope.components.extend(next.scope.components);
                    item.scope.contracts.extend(next.scope.contracts);
                    item.scope.resources.extend(next.scope.resources);
                    item.issues.extend(next.issues);
                    item.acceptance_conditions
                        .extend(next.acceptance_conditions);
                    item.dependencies.extend(next.dependencies);
                    item.dependencies.remove(&id);
                    item.dependencies.remove(&next.id);
                    for other in &mut plan.items {
                        if other.dependencies.remove(&next.id) && other.id != id {
                            other.dependencies.insert(id.clone());
                        }
                    }
                    if let Err(error) = plan.validate() {
                        *plan = original;
                        self.notice = error;
                    }
                    self.start_id = None;
                    None
                }
                KeyCode::Char('x') if self.plan_item.is_none() => {
                    if let Some(item) = plan.items.get_mut(self.cursor) {
                        if item.issues.len() > 1 {
                            let issue = item.issues.pop().expect("multiple issues");
                            let mut split = item.clone();
                            split.id = format!("issue-{}", issue.number);
                            split.objective = issue.title.clone();
                            split.issues = vec![issue];
                            split.dependencies.insert(item.id.clone());
                            plan.items.insert(self.cursor + 1, split);
                            self.start_id = None;
                        }
                    }
                    None
                }
                _ => None,
            },
            Screen::Assignments(_) => {
                if key.code == KeyCode::Char('/') {
                    self.filter = true;
                    self.search.set_input_active(true);
                    return Outcome::None;
                }
                if key.code == KeyCode::Char('r') {
                    Some(Request::List)
                } else if let Some(view) = self.filtered_assignments().get(self.cursor).copied() {
                    if key.code == KeyCode::Enter {
                        if let Some(thread) = &view.assignment.thread_id {
                            if let Ok(id) = zeta_protocol::SessionId::new(thread.to_string()) {
                                return Outcome::OpenWork(id);
                            }
                        }
                        return Outcome::None;
                    }
                    let action = match key.code {
                        KeyCode::Char('u') => Some(Action::Release),
                        KeyCode::Char('y') => Some(Action::RetrySync),
                        KeyCode::Char('t') => {
                            self.transfer = Some((
                                view.assignment.id.clone(),
                                view.assignment.revision,
                                view.assignment.epoch,
                            ));
                            self.edit = Some(self.cursor);
                            self.search.set_query(String::new());
                            self.search.set_input_active(true);
                            return Outcome::None;
                        }
                        _ => None,
                    };
                    action.map(|action| Request::Act {
                        command_id: crate::client::new_command_id("issue-action"),
                        id: view.assignment.id.clone(),
                        revision: view.assignment.revision,
                        epoch: view.assignment.epoch,
                        action,
                    })
                } else {
                    None
                }
            }
            _ => None,
        };
        if let Some(request) = request {
            self.begin();
            Outcome::Request(request)
        } else {
            Outcome::None
        }
    }

    fn filtered_assignments(&self) -> Vec<&View> {
        let query = self.search.query().to_lowercase();
        match &self.screen {
            Screen::Assignments(assignments) => assignments
                .iter()
                .filter(|view| {
                    format!(
                        "{:?} {} {} {} {} {}",
                        view.stage,
                        view.assignment.owner,
                        view.assignment
                            .agent_role
                            .as_ref()
                            .map(|role| role.name.as_str())
                            .unwrap_or(&view.assignment.item.agent),
                        view.assignment.batch_id,
                        view.health,
                        view.assignment
                            .item
                            .issues
                            .iter()
                            .map(|issue| format!("#{} {}", issue.number, issue.title))
                            .collect::<Vec<_>>()
                            .join(" ")
                    )
                    .to_lowercase()
                    .contains(&query)
                })
                .collect(),
            _ => Vec::new(),
        }
    }
    fn set_field(&mut self, index: usize, value: String) -> Result<Option<Request>, String> {
        match &mut self.screen {
            Screen::Workflow(Some(workflow)) => {
                set_workflow_field(&mut workflow.settings, index, &value)?;
                Ok(None)
            }
            Screen::Plan(Some(plan)) => {
                let item_index = self.plan_item.ok_or("Select a work item")?;
                let mut candidate = plan.clone();
                let item = &mut candidate.items[item_index];
                match index {
                    0 => item.objective = value,
                    1 => item.agent = value,
                    2 => {
                        item.acceptance_conditions = value
                            .split(';')
                            .map(str::trim)
                            .filter(|value| !value.is_empty())
                            .map(str::to_owned)
                            .collect()
                    }
                    3 => item.dependencies = words(&value),
                    4 => item.scope.paths = words(&value),
                    5 => item.scope.contracts = words(&value),
                    6 => item.scope.resources = words(&value),
                    _ => return Err("Unknown work item field".into()),
                }
                candidate.validate()?;
                *plan = candidate;
                self.start_id = None;
                Ok(None)
            }
            Screen::Assignments(_) => {
                let (id, revision, epoch) = self.transfer.clone().ok_or("Select an assignment")?;
                if value.trim().is_empty() {
                    return Err("Enter a GitHub account".into());
                }
                Ok(Some(Request::Act {
                    command_id: crate::client::new_command_id("issue-transfer"),
                    id,
                    revision,
                    epoch,
                    action: Action::Transfer(value.trim().into()),
                }))
            }
            _ => Err("No editable field".into()),
        }
    }
    fn rows(&self) -> Vec<String> {
        match &self.screen {
            Screen::Workflow(Some(workflow)) => workflow_fields(&workflow.settings)
                .into_iter()
                .map(|(label, value)| format!("{label}: {value}"))
                .collect(),
            Screen::Plan(Some(plan)) if self.plan_item.is_some() => plan
                .items
                .get(self.plan_item.expect("selected item"))
                .map(|item| {
                    item_fields(item)
                        .into_iter()
                        .map(|(name, value)| format!("{name}: {value}"))
                        .collect()
                })
                .unwrap_or_default(),
            Screen::Plan(Some(plan)) => plan
                .items
                .iter()
                .map(|item| {
                    format!(
                        "{} · {} · {} · depends: {}",
                        item.id,
                        item.issues
                            .iter()
                            .map(|issue| format!("#{}", issue.number))
                            .collect::<Vec<_>>()
                            .join(","),
                        item.objective,
                        item.dependencies
                            .iter()
                            .cloned()
                            .collect::<Vec<_>>()
                            .join(",")
                    )
                })
                .collect(),
            Screen::Assignments(_) => self
                .filtered_assignments()
                .iter()
                .map(|view| {
                    format!(
                        "{}  @{} / {}  {:?} · {} · {}",
                        view.assignment
                            .item
                            .issues
                            .iter()
                            .map(|issue| format!("#{}", issue.number))
                            .collect::<Vec<_>>()
                            .join(","),
                        view.assignment.owner,
                        view.assignment
                            .agent_role
                            .as_ref()
                            .map(|role| role.name.as_str())
                            .unwrap_or(&view.assignment.item.agent),
                        view.stage,
                        if self.busy { "Updating" } else { &view.health },
                        view.assignment
                            .execution_error
                            .as_deref()
                            .unwrap_or(&view.assignment.detail)
                    )
                })
                .collect(),
            _ => Vec::new(),
        }
    }
    pub(crate) fn draw(&self, frame: &mut Frame<'_>, area: Rect, context: RenderContext<'_>) {
        panel::draw_header(
            frame,
            area,
            match self.screen {
                Screen::Workflow(_) => "Issue workflow · current repository",
                Screen::Plan(_) => "Review issue assignments",
                _ => "Issue details",
            },
            context.focus(),
        );
        let area = PanelLayout::new(area, 0).body;
        let style = Style::default()
            .fg(context.foreground())
            .bg(context.background());
        if self.details {
            let mut text = format!(
                "{}\n\nEnter: open work · /: filter · r: refresh\np: pause · s: resume · P: pause batch · C: cancel batch\nt: transfer · u: release ownership · c: cancel work\ny: retry synchronization · v: verify · d: deliver\n\n",
                self.notice
            );
            if let Some(view) = self.filtered_assignments().get(self.cursor) {
                text.push_str(&format!("{}\n\n{}\n{}\n\nBranch: {}\nTarget: {}\nBatch: {}\n\nAcceptance:\n{}\n\nValidation commands:\n{}\n", view.assignment.item.objective, view.assignment.detail, view.assignment.execution_error.as_deref().unwrap_or(""), view.assignment.branch, view.assignment.target_branch, view.assignment.batch_id, view.assignment.item.acceptance_conditions.join("\n"), view.assignment.workflow.validation_commands.join("\n")));
                if let Some(receipt) = &view.assignment.delivery {
                    text.push_str(&format!(
                        "\nCommit: {}\nVerification: {}\n{}",
                        receipt.commit,
                        receipt.verification_key,
                        receipt.pull_request_url.as_deref().unwrap_or("")
                    ));
                }
            }
            text.push_str(&format!("\n\n{}", self.material));
            frame.render_widget(
                Paragraph::new(text)
                    .style(style)
                    .wrap(ratatui::widgets::Wrap { trim: false })
                    .scroll((self.detail_scroll, 0)),
                area,
            );
            return;
        }
        let mut lines = Vec::new();
        if let Screen::Workflow(Some(workflow)) = &self.screen {
            lines.push(Line::from(format!(
                "{}/{} · config {} · default {} · {} labels",
                workflow.repository.owner,
                workflow.repository.name,
                workflow.revision,
                workflow.default_branch,
                workflow.labels.len()
            )));
        }
        if let Screen::Plan(Some(plan)) = &self.screen {
            lines.push(Line::from(format!(
                "{} workers · {} tokens · base {} → {}",
                plan.workflow.max_parallel,
                plan.workflow.max_tokens,
                &plan.base_commit[..plan.base_commit.len().min(12)],
                plan.target_branch
            )));
        }
        if let Screen::Plan(Some(plan)) = &self.screen {
            if let Some(item) = plan.items.get(self.plan_item.unwrap_or(self.cursor)) {
                lines.push(Line::from(format!(
                    "@{} / {} · {:?} · {:?}",
                    plan.workflow.assignee,
                    if item.agent.is_empty() {
                        "automatic Agent"
                    } else {
                        &item.agent
                    },
                    plan.workflow.delivery,
                    plan.workflow.publication
                )));
                lines.push(Line::from(format!(
                    "Scope: {} · APIs: {} · resources: {}",
                    item.scope
                        .paths
                        .iter()
                        .cloned()
                        .collect::<Vec<_>>()
                        .join(","),
                    item.scope
                        .contracts
                        .iter()
                        .cloned()
                        .collect::<Vec<_>>()
                        .join(","),
                    item.scope
                        .resources
                        .iter()
                        .cloned()
                        .collect::<Vec<_>>()
                        .join(",")
                )));
                lines.push(Line::from(format!(
                    "Acceptance: {}",
                    item.acceptance_conditions.join("; ")
                )));
                lines.push(Line::from(format!(
                    "Validation: {}",
                    plan.workflow.validation_commands.join("; ")
                )));
            }
        }
        if let Some(view) = self.filtered_assignments().get(self.cursor) {
            lines.push(Line::from(view.assignment.item.objective.clone()));
            lines.push(Line::from(format!(
                "Owner @{} · Agent {} · {:?} · sync {:?}",
                view.assignment.owner,
                view.assignment
                    .agent_role
                    .as_ref()
                    .map(|role| role.name.as_str())
                    .unwrap_or(&view.assignment.item.agent),
                view.stage,
                view.assignment.sync_state
            )));
            lines.push(Line::from(format!(
                "{} → {}",
                view.assignment.branch, view.assignment.target_branch
            )));
            if let Some(receipt) = &view.assignment.delivery {
                lines.push(Line::from(format!(
                    "Result {} · {}",
                    receipt.commit,
                    receipt.pull_request_url.as_deref().unwrap_or("no PR")
                )));
            }
        }
        if !self.notice.is_empty() {
            lines.push(Line::from(self.notice.clone()));
        }
        if self.edit.is_some() || self.filter {
            lines.push(Line::from(format!("> {}", self.search.query())));
        }
        let rows = self.rows();
        let available = usize::from(area.height).saturating_sub(lines.len() + 1);
        let offset = self.cursor.saturating_sub(available.saturating_sub(1));
        for (index, row) in rows.iter().enumerate().skip(offset).take(available) {
            lines.push(Line::styled(
                format!("{}{}", if index == self.cursor { "> " } else { "  " }, row),
                if index == self.cursor {
                    style.fg(context.focus())
                } else {
                    style
                },
            ));
        }
        if rows.is_empty() && !self.busy {
            lines.push(Line::from("No matching assignments"));
        }
        if matches!(self.screen, Screen::Assignments(_))
            && !self.material.is_empty()
            && self.edit.is_none()
        {
            lines.push(Line::from("Issue contents · i for full details"));
            let remaining = usize::from(area.height).saturating_sub(lines.len());
            lines.extend(
                self.material
                    .lines()
                    .skip(2)
                    .take(remaining)
                    .map(|line| Line::from(line.to_owned())),
            );
        }
        frame.render_widget(Paragraph::new(lines).style(style), area);
    }
}

fn workflow_fields(settings: &IssueWorkflow) -> Vec<(&'static str, String)> {
    let mut fields = vec![
        ("Todo label", settings.labels.todo.clone()),
        ("Queued label", settings.labels.queued.clone()),
        ("In progress label", settings.labels.in_progress.clone()),
        ("Review label", settings.labels.review.clone()),
        ("Blocked label", settings.labels.blocked.clone()),
        ("GitHub assignee", settings.assignee.clone()),
        (
            "Base branch (empty = default, HEAD = checkout)",
            settings.base_branch.clone().unwrap_or_default(),
        ),
        (
            "Target branch (empty = default)",
            settings.target_branch.clone().unwrap_or_default(),
        ),
        ("Branch template", settings.branch_template.clone()),
        (
            "Branch publication (local/linked)",
            serde_json::to_value(settings.publication)
                .expect("enum")
                .as_str()
                .expect("enum string")
                .into(),
        ),
        ("Coordinator Agent", settings.coordinator_agent.clone()),
        ("Worker Agent", settings.worker_agent.clone()),
        ("Parallel workers", settings.max_parallel.to_string()),
        ("Batch token budget", settings.max_tokens.to_string()),
        (
            "Delivery (branch/pullRequest/draftPullRequest)",
            serde_json::to_value(settings.delivery)
                .expect("enum")
                .as_str()
                .expect("enum string")
                .into(),
        ),
        (
            "Close when complete (true/false)",
            settings.close_on_completion.to_string(),
        ),
        (
            "Validation commands (paste one per line)",
            settings.validation_commands.join("\n"),
        ),
        (
            "Automatic claiming (true/false)",
            settings.auto_claim.is_some().to_string(),
        ),
        (
            "Auto-claim labels (comma separated)",
            settings
                .auto_claim
                .as_ref()
                .map(|rule| rule.labels.join(", "))
                .unwrap_or_default(),
        ),
        (
            "Auto-claim assignee (empty = unassigned)",
            settings
                .auto_claim
                .as_ref()
                .and_then(|rule| rule.assignee.clone())
                .unwrap_or_default(),
        ),
        (
            "Auto-claim issue limit",
            settings
                .auto_claim
                .as_ref()
                .map(|rule| rule.max_issues.to_string())
                .unwrap_or_default(),
        ),
    ];
    fields.push((
        "Automatic PR merge (true/false)",
        settings.auto_merge.to_string(),
    ));
    for ((title, default), name) in [
        ("Todo color", "cfd3d7"),
        ("Queued color", "d4c5f9"),
        ("In progress color", "1d76db"),
        ("Review color", "0e8a16"),
        ("Blocked color", "b60205"),
    ]
    .into_iter()
    .zip(settings.labels.all())
    {
        fields.push((
            title,
            settings
                .label_colors
                .get(name)
                .cloned()
                .unwrap_or_else(|| default.into()),
        ));
    }
    fields
}
fn set_workflow_field(
    settings: &mut IssueWorkflow,
    index: usize,
    value: &str,
) -> Result<(), String> {
    let mut candidate = settings.clone();
    match index {
        0 => candidate.labels.todo = value.into(),
        1 => candidate.labels.queued = value.into(),
        2 => candidate.labels.in_progress = value.into(),
        3 => candidate.labels.review = value.into(),
        4 => candidate.labels.blocked = value.into(),
        5 => candidate.assignee = value.into(),
        6 => candidate.base_branch = (!value.is_empty()).then(|| value.into()),
        7 => candidate.target_branch = (!value.is_empty()).then(|| value.into()),
        8 => candidate.branch_template = value.into(),
        9 => {
            candidate.publication = serde_json::from_value(serde_json::json!(value))
                .map_err(|error| error.to_string())?
        }
        10 => candidate.coordinator_agent = value.into(),
        11 => candidate.worker_agent = value.into(),
        12 => candidate.max_parallel = value.parse().map_err(|_| "Enter a worker count")?,
        13 => candidate.max_tokens = value.parse().map_err(|_| "Enter a token budget")?,
        14 => {
            candidate.delivery = serde_json::from_value(serde_json::json!(value))
                .map_err(|error| error.to_string())?
        }
        15 => candidate.close_on_completion = value.parse().map_err(|_| "Enter true or false")?,
        16 => {
            candidate.validation_commands = value
                .lines()
                .map(str::trim)
                .filter(|line| !line.is_empty())
                .map(str::to_owned)
                .collect()
        }
        17 => {
            candidate.auto_claim = if value.parse::<bool>().map_err(|_| "Enter true or false")? {
                Some(github::IssueAutoClaim {
                    labels: vec![candidate.labels.todo.clone()],
                    assignee: None,
                    max_issues: 1,
                })
            } else {
                None
            };
        }
        18 => {
            candidate
                .auto_claim
                .as_mut()
                .ok_or("Enable automatic claiming first")?
                .labels = words(value).into_iter().collect()
        }
        19 => {
            candidate
                .auto_claim
                .as_mut()
                .ok_or("Enable automatic claiming first")?
                .assignee = (!value.is_empty()).then(|| value.into())
        }
        20 => {
            candidate
                .auto_claim
                .as_mut()
                .ok_or("Enable automatic claiming first")?
                .max_issues = value.parse().map_err(|_| "Enter an issue limit")?
        }
        21 => candidate.auto_merge = value.parse().map_err(|_| "Enter true or false")?,
        22..=26 => {
            let name = candidate.labels.all()[index - 22].to_owned();
            candidate.label_colors.insert(name, value.into());
        }
        _ => return Err("Unknown workflow field".into()),
    }
    if index < 5 {
        let old = settings.labels.all()[index];
        if let Some(color) = candidate.label_colors.remove(old) {
            candidate.label_colors.insert(value.into(), color);
        }
    }
    candidate.validate()?;
    *settings = candidate;
    Ok(())
}

fn words(value: &str) -> std::collections::BTreeSet<String> {
    value
        .split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
        .collect()
}
fn item_fields(item: &github::IssueWorkItem) -> Vec<(&'static str, String)> {
    vec![
        ("Objective", item.objective.clone()),
        ("Agent", item.agent.clone()),
        (
            "Acceptance conditions (separate with ;)",
            item.acceptance_conditions.join("; "),
        ),
        (
            "Depends on work items",
            item.dependencies
                .iter()
                .cloned()
                .collect::<Vec<_>>()
                .join(", "),
        ),
        (
            "Expected paths",
            item.scope
                .paths
                .iter()
                .cloned()
                .collect::<Vec<_>>()
                .join(", "),
        ),
        (
            "Shared interfaces",
            item.scope
                .contracts
                .iter()
                .cloned()
                .collect::<Vec<_>>()
                .join(", "),
        ),
        (
            "Shared resources",
            item.scope
                .resources
                .iter()
                .cloned()
                .collect::<Vec<_>>()
                .join(", "),
        ),
    ]
}

#[cfg(test)]
#[path = "assignment_tests.rs"]
mod tests;
