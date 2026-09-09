use super::Issue;
use super::IssueState;
use super::assignment::View;
use crate::render::InteractionState;
use crate::render::InteractionTarget;
use crate::render::RenderContext;
use crate::render::interaction_style;
use crate::widgets::grouped_list::more_line;
use crate::widgets::grouped_list::pad_to_width;
use crate::widgets::grouped_list::truncate_to_width;
use crate::widgets::navigation::Navigation;
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::Modifier;
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::widgets::Paragraph;
use std::collections::BTreeMap;
use std::collections::BTreeSet;
use github::IssueLabels;
use github::IssueOwnership;
use github::IssueStage;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(super) enum Group {
    Todo,
    Queued,
    InProgress,
    Review,
    Blocked,
    Delivered,
    Closed,
}
impl Group {
    const ALL: [Self; 7] = [
        Self::Todo,
        Self::Queued,
        Self::InProgress,
        Self::Review,
        Self::Blocked,
        Self::Delivered,
        Self::Closed,
    ];
    pub(super) fn label(self) -> &'static str {
        match self {
            Self::Todo => "Todo",
            Self::Queued => "Queued",
            Self::InProgress => "In progress",
            Self::Review => "Review",
            Self::Blocked => "Blocked",
            Self::Delivered => "Delivered",
            Self::Closed => "Closed",
        }
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) enum Target {
    Group(Group),
    Issue(u64),
}
#[derive(Clone, Debug)]
pub(super) struct Entry {
    pub issue: Issue,
    pub group: Group,
    pub work: Option<View>,
}
impl Entry {
    pub(super) fn active_work(&self) -> Option<&View> {
        self.work.as_ref().filter(|view| {
            !matches!(
                view.assignment.ownership,
                IssueOwnership::Released | IssueOwnership::Completed
            )
        })
    }
    fn search_text(&self) -> String {
        format!(
            "#{} {} {} {} {}",
            self.issue.number,
            self.issue.title,
            format!(
                "{} {}",
                self.issue
                    .assignees
                    .iter()
                    .map(|owner| format!("@{owner}"))
                    .collect::<Vec<_>>()
                    .join(" "),
                self.issue.labels.join(" ")
            ),
            self.group.label(),
            self.work
                .as_ref()
                .map(|view| format!(
                    "{} {} {} {:?} {}",
                    view.assignment.owner,
                    view.assignment.item.agent,
                    view.assignment.batch_id,
                    view.assignment.sync_state,
                    view.health
                ))
                .unwrap_or_default()
        )
        .to_lowercase()
    }
}
#[derive(Clone, Debug)]
pub(super) enum Row {
    Group(Group, usize),
    Issue(Entry),
}
impl Row {
    fn target(&self) -> Target {
        match self {
            Self::Group(group, _) => Target::Group(*group),
            Self::Issue(entry) => Target::Issue(entry.issue.number),
        }
    }
}

#[derive(Debug)]
pub(super) struct Board {
    raw_at: BTreeMap<u64, u64>,
    open: Vec<Issue>,
    closed: Vec<Issue>,
    pub views: Vec<View>,
    pub labels: Option<IssueLabels>,
    pub selected: Option<Target>,
    collapsed: BTreeSet<Group>,
    open_next: Option<u32>,
    closed_next: Option<u32>,
    pub closed_loaded: bool,
}
impl Default for Board {
    fn default() -> Self {
        Self {
            raw_at: BTreeMap::new(),
            open: Vec::new(),
            closed: Vec::new(),
            views: Vec::new(),
            labels: None,
            selected: None,
            collapsed: BTreeSet::from([Group::Closed, Group::Delivered]),
            open_next: None,
            closed_next: None,
            closed_loaded: false,
        }
    }
}
impl Board {
    pub fn loaded(&self) -> usize {
        self.open.len() + self.closed.len()
    }
    pub fn next(&self, state: IssueState) -> Option<u32> {
        if state == IssueState::Closed {
            self.closed_next
        } else {
            self.open_next
        }
    }
    pub fn clear_pages(&mut self) {
        self.raw_at.clear();
        self.open.clear();
        self.closed.clear();
        self.open_next = None;
        self.closed_next = None;
        self.closed_loaded = false;
    }
    pub fn page(
        &mut self,
        state: IssueState,
        page: u32,
        issues: Vec<Issue>,
        next: Option<u32>,
        fetched_at: u64,
    ) {
        let issues = issues
            .into_iter()
            .filter(|issue| {
                self.raw_at
                    .get(&issue.number)
                    .is_none_or(|known| *known <= fetched_at)
            })
            .collect::<Vec<_>>();
        for issue in &issues {
            self.raw_at.insert(issue.number, fetched_at);
        }
        let other = if state == IssueState::Closed {
            &mut self.open
        } else {
            &mut self.closed
        };
        other.retain(|old| !issues.iter().any(|new| old.number == new.number));
        let target = if state == IssueState::Closed {
            self.closed_loaded = true;
            self.closed_next = next;
            &mut self.closed
        } else {
            self.open_next = next;
            &mut self.open
        };
        if page == 1 {
            target.clear();
        }
        for issue in issues {
            if let Some(old) = target.iter_mut().find(|old| old.number == issue.number) {
                *old = issue;
            } else {
                target.push(issue);
            }
        }
    }
    pub fn entries(&self, query: &str, editing: bool) -> Vec<Entry> {
        let mut entries = BTreeMap::new();
        for (issues, closed) in [(&self.open, false), (&self.closed, true)] {
            for issue in issues {
                let mut group = if issue.assignees.is_empty() {
                    Group::Todo
                } else {
                    Group::Queued
                };
                if let Some(labels) = &self.labels {
                    for (name, stage) in [
                        (&labels.queued, Group::Queued),
                        (&labels.in_progress, Group::InProgress),
                        (&labels.review, Group::Review),
                        (&labels.blocked, Group::Blocked),
                    ] {
                        if issue.labels.contains(name) {
                            group = stage;
                        }
                    }
                }
                if closed {
                    group = Group::Closed;
                }
                entries.insert(
                    issue.number,
                    Entry {
                        issue: issue.clone(),
                        group,
                        work: None,
                    },
                );
            }
        }
        // Newest active ownership wins over historical released/completed work.
        let mut works = BTreeMap::<u64, &View>::new();
        for view in &self.views {
            for issue in &view.assignment.item.issues {
                let active = |view: &View| {
                    !matches!(
                        view.assignment.ownership,
                        IssueOwnership::Released | IssueOwnership::Completed
                    )
                };
                if works
                    .get(&issue.number)
                    .is_none_or(|old| !active(old) && active(view))
                {
                    works.insert(issue.number, view);
                }
            }
        }
        let query = query.to_lowercase();
        for (number, view) in works {
            let identity = view
                .assignment
                .item
                .issues
                .iter()
                .find(|issue| issue.number == number)
                .expect("owned issue");
            let mut entry = entries.remove(&number).unwrap_or_else(|| Entry {
                issue: Issue {
                    number,
                    title: identity.title.clone(),
                    labels: Vec::new(),
                    assignees: Vec::new(),
                },
                group: Group::Todo,
                work: None,
            });
            let was_loaded = self
                .open
                .iter()
                .chain(&self.closed)
                .any(|issue| issue.number == number);
            let terminal_is_history = was_loaded
                && matches!(
                    view.assignment.ownership,
                    IssueOwnership::Released | IssueOwnership::Completed
                )
                && self
                    .raw_at
                    .get(&number)
                    .is_some_and(|time| *time > view.assignment.updated_at);
            if entry.group != Group::Closed && !terminal_is_history {
                entry.group = match view.stage {
                    IssueStage::Todo => Group::Todo,
                    IssueStage::Queued => Group::Queued,
                    IssueStage::InProgress => Group::InProgress,
                    IssueStage::Review => Group::Review,
                    IssueStage::Blocked | IssueStage::Cancelled => Group::Blocked,
                    IssueStage::Completed => {
                        if view.assignment.workflow.close_on_completion {
                            Group::Closed
                        } else {
                            Group::Delivered
                        }
                    }
                };
            }
            if !terminal_is_history {
                match view.assignment.ownership {
                    IssueOwnership::Unclaimed => {}
                    IssueOwnership::Released => {
                        entry
                            .issue
                            .assignees
                            .retain(|owner| !owner.eq_ignore_ascii_case(&view.assignment.owner));
                        if entry.group != Group::Closed && !entry.issue.assignees.is_empty() {
                            entry.group = Group::Queued;
                        }
                    }
                    _ => {
                        entry.issue.assignees = if view.assignment.owner.is_empty() {
                            Vec::new()
                        } else {
                            vec![view.assignment.owner.clone()]
                        };
                    }
                }
            }
            entry.work = Some(view.clone());
            if query.is_empty() || was_loaded || entry.search_text().contains(&query) {
                entries.insert(number, entry);
            }
        }
        entries
            .into_values()
            .filter(|entry| !editing || entry.search_text().contains(&query))
            .collect()
    }
    pub fn rows(&self, query: &str, editing: bool) -> Vec<Row> {
        let entries = self.entries(query, editing);
        let mut rows = Vec::new();
        for group in Group::ALL {
            let members = entries
                .iter()
                .filter(|entry| entry.group == group)
                .collect::<Vec<_>>();
            if members.is_empty() && group != Group::Closed {
                continue;
            }
            rows.push(Row::Group(group, members.len()));
            if !self.collapsed.contains(&group) {
                rows.extend(members.into_iter().cloned().map(Row::Issue));
            }
        }
        rows
    }
    pub fn reconcile(&mut self, query: &str, editing: bool) {
        let entries = self.entries(query, editing);
        if let Some(Target::Issue(number)) = self.selected {
            if let Some(entry) = entries.iter().find(|entry| entry.issue.number == number) {
                self.collapsed.remove(&entry.group);
            }
        }
        let rows = self.rows(query, editing);
        if self
            .selected
            .as_ref()
            .is_none_or(|target| !rows.iter().any(|row| &row.target() == target))
        {
            self.selected = rows
                .iter()
                .find(|row| matches!(row, Row::Issue(_)))
                .or(rows.first())
                .map(Row::target);
        }
    }
    pub fn jump_group(&mut self, direction: isize, query: &str) {
        let rows = self.rows(query, false);
        let index = self
            .selected
            .as_ref()
            .and_then(|target| rows.iter().position(|row| &row.target() == target))
            .unwrap_or_default();
        let groups = rows
            .iter()
            .enumerate()
            .filter(|(_, row)| matches!(row, Row::Group(..)))
            .collect::<Vec<_>>();
        let next = if direction > 0 {
            groups.iter().find(|(at, _)| *at > index).or(groups.first())
        } else {
            groups
                .iter()
                .rev()
                .find(|(at, _)| *at < index)
                .or(groups.last())
        };
        self.selected = next.map(|(_, row)| row.target());
    }
    pub fn navigate(&mut self, navigation: Navigation, query: &str, editing: bool) {
        let rows = self.rows(query, editing);
        let index = self
            .selected
            .as_ref()
            .and_then(|target| rows.iter().position(|row| &row.target() == target))
            .unwrap_or_default();
        let next = navigation.offset(index, rows.len().saturating_sub(1), 12);
        self.selected = rows.get(next).map(Row::target);
    }
    pub fn selected_entry(&self, query: &str, editing: bool) -> Option<Entry> {
        let Some(Target::Issue(number)) = self.selected else {
            return None;
        };
        self.entries(query, editing)
            .into_iter()
            .find(|entry| entry.issue.number == number)
    }
    pub fn selected_state(&self) -> IssueState {
        match &self.selected {
            Some(Target::Group(Group::Closed)) => IssueState::Closed,
            Some(Target::Issue(number))
                if self
                    .entries("", false)
                    .iter()
                    .any(|entry| entry.issue.number == *number && entry.group == Group::Closed) =>
            {
                IssueState::Closed
            }
            _ => IssueState::Open,
        }
    }
    pub fn toggle(&mut self, group: Group) {
        if !self.collapsed.remove(&group) {
            self.collapsed.insert(group);
        }
        self.selected = Some(Target::Group(group));
    }
    pub fn expand(&mut self, group: Group) {
        self.collapsed.remove(&group);
    }
    pub fn collapse(&mut self, group: Group) {
        self.collapsed.insert(group);
        self.selected = Some(Target::Group(group));
    }
    pub fn collapsed(&self, group: Group) -> bool {
        self.collapsed.contains(&group)
    }
    pub fn draw(
        &self,
        frame: &mut Frame<'_>,
        area: Rect,
        query: &str,
        editing: bool,
        selected: &BTreeSet<u64>,
        context: RenderContext<'_>,
    ) {
        let rows = self.rows(query, editing);
        let index = self
            .selected
            .as_ref()
            .and_then(|target| rows.iter().position(|row| &row.target() == target));
        let viewport =
            crate::widgets::grouped_list::viewport(rows.len(), index, usize::from(area.height));
        let mut lines = Vec::new();
        let muted = Style::default().fg(context.muted());
        if viewport.start > 0 {
            lines.push(more_line(
                '↑',
                viewport.start,
                usize::from(area.width),
                context,
            ));
        }
        for row in &rows[viewport.start..viewport.end] {
            let focused = !editing && Some(&row.target()) == self.selected.as_ref();
            let interaction = interaction_style(
                context,
                InteractionState {
                    target: InteractionTarget::Rest,
                    selected: focused,
                    hovered: false,
                    pressed: false,
                },
            );
            let style = muted.patch(interaction);
            lines.push(match row {
                Row::Group(group, count) => Line::styled(
                    pad_to_width(
                        &truncate_to_width(
                            &format!(
                                "{}{} {} ({count}){}",
                                crate::render::selection_marker(focused),
                                if self.collapsed(*group) { '▸' } else { '▾' },
                                group.label(),
                                if *group == Group::Closed && !self.closed_loaded {
                                    " · expand to load"
                                } else {
                                    ""
                                }
                            ),
                            usize::from(area.width),
                        ),
                        usize::from(area.width),
                    ),
                    muted.add_modifier(Modifier::BOLD).patch(interaction),
                ),
                Row::Issue(entry) => {
                    let agent = entry
                        .work
                        .as_ref()
                        .map(|view| {
                            view.assignment
                                .agent_role
                                .as_ref()
                                .map(|role| role.name.as_str())
                                .unwrap_or(&view.assignment.item.agent)
                        })
                        .unwrap_or("");
                    let owner = entry
                        .issue
                        .assignees
                        .iter()
                        .map(|owner| format!("@{owner}"))
                        .collect::<Vec<_>>()
                        .join(", ");
                    let metadata = format!(
                        "{owner}{}",
                        if agent.is_empty() {
                            String::new()
                        } else {
                            format!(" / {agent}")
                        }
                    );
                    let health = entry
                        .work
                        .as_ref()
                        .map(|view| view.health.as_str())
                        .unwrap_or("");
                    let available = usize::from(area.width).saturating_sub(9);
                    let name_width = if available >= 40 {
                        available * 55 / 100
                    } else {
                        available
                    };
                    let owner_width = if available >= 40 {
                        available * 25 / 100
                    } else {
                        0
                    };
                    let health_width = available.saturating_sub(name_width + owner_width);
                    let title = format!(
                        "#{} {}",
                        entry.issue.number,
                        entry.issue.title.replace(['\n', '\r'], " ")
                    );
                    Line::styled(
                        pad_to_width(
                            &format!(
                                "{}  [{}] {}{}{}",
                                crate::render::selection_marker(focused),
                                if selected.contains(&entry.issue.number) {
                                    "x"
                                } else {
                                    " "
                                },
                                pad_to_width(&truncate_to_width(&title, name_width), name_width),
                                pad_to_width(
                                    &truncate_to_width(&metadata, owner_width),
                                    owner_width
                                ),
                                truncate_to_width(health, health_width)
                            ),
                            usize::from(area.width),
                        ),
                        style,
                    )
                }
            });
        }
        if viewport.end < rows.len() {
            lines.push(more_line(
                '↓',
                rows.len() - viewport.end,
                usize::from(area.width),
                context,
            ));
        }
        frame.render_widget(Paragraph::new(lines), area);
    }
}
