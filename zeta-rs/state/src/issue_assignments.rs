use crate::SqliteDurability;
use crate::open_sqlite_database;
use github::IssueAssignment;
use github::IssueAssignmentCommand;
use github::IssueAssignmentPlan;
use github::IssueControl;
use github::IssueOwnership;
use github::IssueSyncState;
use rusqlite::Connection;
use rusqlite::OptionalExtension;
use serde::Serialize;
use sha2::Digest;
use sha2::Sha256;
use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Mutex;

/// SQLite implementation of GitHub Issue claims and idempotent operation receipts.
pub struct SqliteIssueAssignmentStore {
    connection: Mutex<Connection>,
}

impl SqliteIssueAssignmentStore {
    pub fn open(path: &Path) -> Result<Self, String> {
        let connection = open_sqlite_database(path, SqliteDurability::Durable)?;
        connection.execute_batch("CREATE TABLE IF NOT EXISTS issue_assignments(id TEXT PRIMARY KEY, repository TEXT NOT NULL, batch TEXT NOT NULL, revision INTEGER NOT NULL, value TEXT NOT NULL);
            CREATE TABLE IF NOT EXISTS issue_claims(repository TEXT NOT NULL, issue TEXT NOT NULL, assignment TEXT NOT NULL REFERENCES issue_assignments(id), PRIMARY KEY(repository,issue));
            CREATE TABLE IF NOT EXISTS issue_action_receipts(id TEXT PRIMARY KEY, fingerprint TEXT NOT NULL, value TEXT NOT NULL);
            CREATE TABLE IF NOT EXISTS issue_assignment_receipts(id TEXT PRIMARY KEY, fingerprint TEXT NOT NULL, value TEXT NOT NULL);
            CREATE INDEX IF NOT EXISTS issue_assignments_repository ON issue_assignments(repository);
            CREATE TABLE IF NOT EXISTS issue_auto_claim(repository TEXT PRIMARY KEY, activation TEXT NOT NULL, enabled INTEGER NOT NULL, source TEXT NOT NULL DEFAULT '');")
            .map_err(|error| error.to_string())?;
        let has_source = {
            let mut statement = connection
                .prepare("PRAGMA table_info(issue_auto_claim)")
                .map_err(|error| error.to_string())?;
            let columns = statement
                .query_map([], |row| row.get::<_, String>(1))
                .map_err(|error| error.to_string())?;
            columns
                .collect::<Result<Vec<_>, _>>()
                .map_err(|error| error.to_string())?
                .iter()
                .any(|name| name == "source")
        };
        if !has_source {
            connection
                .execute(
                    "ALTER TABLE issue_auto_claim ADD COLUMN source TEXT NOT NULL DEFAULT ''",
                    [],
                )
                .map_err(|error| error.to_string())?;
        }
        Ok(Self {
            connection: Mutex::new(connection),
        })
    }

    pub fn action_receipt(
        &self,
        id: &str,
        request: &serde_json::Value,
    ) -> Result<Option<serde_json::Value>, String> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| "Issue store lock poisoned")?;
        let receipt: Option<(String, String)> = connection
            .query_row(
                "SELECT fingerprint,value FROM issue_action_receipts WHERE id=?1",
                [id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()
            .map_err(|error| error.to_string())?;
        receipt
            .map(|(expected, value)| {
                if expected != fingerprint(request)? {
                    return Err("Issue command identity was reused with different input".into());
                }
                serde_json::from_str(&value).map_err(|error| error.to_string())
            })
            .transpose()
    }
    pub fn record_action(
        &self,
        id: &str,
        request: &serde_json::Value,
        response: &serde_json::Value,
    ) -> Result<(), String> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| "Issue store lock poisoned")?;
        connection.execute("INSERT INTO issue_action_receipts(id,fingerprint,value) VALUES(?1,?2,?3) ON CONFLICT(id) DO NOTHING", (id, fingerprint(request)?, response.to_string())).map_err(|error| error.to_string())?;
        Ok(())
    }
    pub fn command_receipt(&self, id: &str) -> Result<Option<IssueAssignment>, String> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| "Issue store lock poisoned")?;
        let value: Option<String> = connection
            .query_row(
                "SELECT value FROM issue_assignment_receipts WHERE id=?1",
                [id],
                |row| row.get(0),
            )
            .optional()
            .map_err(|error| error.to_string())?;
        value
            .map(|value| serde_json::from_str(&value).map_err(|error| error.to_string()))
            .transpose()
    }

    pub fn claim(
        &self,
        command_id: &str,
        plan: &IssueAssignmentPlan,
        now: u64,
    ) -> Result<Vec<IssueAssignment>, String> {
        self.reserve(command_id, plan, now, Reservation::Assignment)
    }

    pub fn prepare_branches(
        &self,
        command_id: &str,
        plan: &IssueAssignmentPlan,
        now: u64,
    ) -> Result<Vec<IssueAssignment>, String> {
        self.reserve(command_id, plan, now, Reservation::Branch)
    }

    pub fn start(
        &self,
        command_id: &str,
        plan: &IssueAssignmentPlan,
        now: u64,
    ) -> Result<Vec<IssueAssignment>, String> {
        self.reserve(command_id, plan, now, Reservation::Execute)
    }

    pub fn existing_batch(&self, command_id: &str) -> Result<Option<Vec<IssueAssignment>>, String> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| "Issue assignment store lock poisoned")?;
        let value: Option<String> = connection
            .query_row(
                "SELECT value FROM issue_assignment_receipts WHERE id=?1",
                [command_id],
                |row| row.get(0),
            )
            .optional()
            .map_err(|error| error.to_string())?;
        value
            .map(|value| {
                serde_json::from_str(&value)
                    .map_err(|error| format!("Command is not an issue batch: {error}"))
            })
            .transpose()
    }

    fn reserve(
        &self,
        command_id: &str,
        plan: &IssueAssignmentPlan,
        now: u64,
        reservation: Reservation,
    ) -> Result<Vec<IssueAssignment>, String> {
        plan.validate()?;
        if command_id.is_empty()
            || command_id.len() > 255
            || reservation != Reservation::Branch && plan.workflow.assignee.is_empty()
        {
            return Err(
                "Claim requires a command identity and a responsible GitHub account".into(),
            );
        }
        let fingerprint = fingerprint(&(reservation, plan))?;
        let mut connection = self
            .connection
            .lock()
            .map_err(|_| "Issue assignment store lock poisoned")?;
        let tx = connection
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(|error| error.to_string())?;
        if let Some(value) = replay(&tx, command_id, &fingerprint)? {
            return serde_json::from_str(&value).map_err(|error| error.to_string());
        }
        let mut conflicts = Vec::new();
        for issue in plan.items.iter().flat_map(|item| &item.issues) {
            let existing: Option<String> = tx
                .query_row(
                    "SELECT assignment FROM issue_claims WHERE repository=?1 AND issue=?2",
                    (plan.repository.key(), &issue.node_id),
                    |row| row.get(0),
                )
                .optional()
                .map_err(|error| error.to_string())?;
            if let Some(id) = existing {
                conflicts.push(format!("#{}: {id}", issue.number));
            }
        }
        if !conflicts.is_empty() {
            return Err(format!("Issues already claimed: {}", conflicts.join(", ")));
        }
        let mut assignments = Vec::new();
        for item in &plan.items {
            let digest = fingerprint_text(&format!("{command_id}:{}", item.id));
            let id = format!("issue-{}", &digest[..24]);
            let branch = plan
                .workflow
                .branch_name(&item.issues[0], &format!("a{}", &digest[..12]))?;
            let assignment = IssueAssignment {
                id: id.clone(),
                batch_id: command_id.into(),
                config_revision: plan.config_revision,
                repository: plan.repository.clone(),
                item: item.clone(),
                workflow: plan.workflow.clone(),
                model: plan.model.clone(),
                agent_role: None,
                agent_tools: Vec::new(),
                planning_tokens: plan.planning_tokens,
                base_commit: plan.base_commit.clone(),
                target_branch: plan.target_branch.clone(),
                branch,
                delivery: None,
                owner: plan.workflow.assignee.clone(),
                pending_owner: None,
                auto_start: reservation == Reservation::Execute,
                revision: 1,
                epoch: 1,
                ownership: if reservation != Reservation::Branch {
                    IssueOwnership::Held
                } else {
                    IssueOwnership::Unclaimed
                },
                thread_id: None,
                lease_until: None,
                sync_state: IssueSyncState::Pending,
                attempted_stages: Vec::new(),
                desired_stage: if reservation == Reservation::Branch {
                    github::IssueStage::Todo
                } else {
                    github::IssueStage::Queued
                },
                paused: false,
                execution_error: None,
                synced_stage: None,
                synced_labels: BTreeMap::new(),
                linked_branch_id: None,
                detail: String::new(),
                updated_at: now,
            };
            tx.execute("INSERT INTO issue_assignments(id,repository,batch,revision,value) VALUES(?1,?2,?3,1,?4)", (&id, plan.repository.key(), command_id, serde_json::to_string(&assignment).map_err(|error| error.to_string())?)).map_err(|error| error.to_string())?;
            for issue in &item.issues {
                if reservation == Reservation::Branch {
                    continue;
                }
                tx.execute(
                    "INSERT INTO issue_claims(repository,issue,assignment) VALUES(?1,?2,?3)",
                    (plan.repository.key(), &issue.node_id, &id),
                )
                .map_err(|error| error.to_string())?;
            }
            assignments.push(assignment);
        }
        record(&tx, command_id, &fingerprint, &assignments)?;
        tx.commit().map_err(|error| error.to_string())?;
        Ok(assignments)
    }

    pub fn read(&self, id: &str) -> Result<IssueAssignment, String> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| "Issue assignment store lock poisoned")?;
        read(&connection, id)
    }

    pub fn set_auto_claim(
        &self,
        repository: &str,
        source: &Path,
        activation: Option<&str>,
    ) -> Result<(), String> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| "Issue assignment store lock poisoned")?;
        match activation {
            Some(activation) => {
                connection.execute("INSERT INTO issue_auto_claim(repository,activation,enabled,source) VALUES(?1,?2,1,?3) ON CONFLICT(repository) DO UPDATE SET activation=excluded.activation,enabled=1,source=excluded.source", (repository, activation, source.to_string_lossy().as_ref())).map_err(|error| error.to_string())?;
            }
            None => {
                connection
                    .execute(
                        "UPDATE issue_auto_claim SET enabled=0 WHERE repository=?1",
                        [repository],
                    )
                    .map_err(|error| error.to_string())?;
            }
        }
        Ok(())
    }

    pub fn auto_claim_for_directory(&self, source: &Path) -> Result<Option<String>, String> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| "Issue assignment store lock poisoned")?;
        connection.query_row("SELECT repository FROM issue_auto_claim WHERE source=?1 AND enabled=1 ORDER BY rowid DESC LIMIT 1", [source.to_string_lossy().as_ref()], |row| row.get(0)).optional().map_err(|error| error.to_string())
    }

    pub fn list_all(&self) -> Result<Vec<IssueAssignment>, String> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| "Issue assignment store lock poisoned")?;
        let mut statement = connection
            .prepare("SELECT value FROM issue_assignments ORDER BY rowid")
            .map_err(|error| error.to_string())?;
        let rows = statement
            .query_map([], |row| row.get::<_, String>(0))
            .map_err(|error| error.to_string())?;
        rows.map(|row| {
            serde_json::from_str(&row.map_err(|error| error.to_string())?)
                .map_err(|error| error.to_string())
        })
        .collect()
    }

    pub fn auto_claim_activation(&self, repository: &str) -> Result<Option<String>, String> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| "Issue assignment store lock poisoned")?;
        connection
            .query_row(
                "SELECT activation FROM issue_auto_claim WHERE repository=?1 AND enabled=1",
                [repository],
                |row| row.get(0),
            )
            .optional()
            .map_err(|error| error.to_string())
    }

    pub fn for_thread(
        &self,
        thread_id: &zeta_protocol::ThreadId,
    ) -> Result<Option<IssueAssignment>, String> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| "Issue assignment store lock poisoned")?;
        let value: Option<String> = connection
            .query_row(
                "SELECT value FROM issue_assignments WHERE json_extract(value,'$.threadId')=?1 ",
                [thread_id.as_str()],
                |row| row.get(0),
            )
            .optional()
            .map_err(|error| error.to_string())?;
        value
            .map(|value| serde_json::from_str(&value).map_err(|error| error.to_string()))
            .transpose()
    }

    pub fn list(&self, repository: &str) -> Result<Vec<IssueAssignment>, String> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| "Issue assignment store lock poisoned")?;
        let mut statement = connection
            .prepare("SELECT value FROM issue_assignments WHERE repository=?1 ORDER BY rowid DESC")
            .map_err(|error| error.to_string())?;
        let rows = statement
            .query_map([repository], |row| row.get::<_, String>(0))
            .map_err(|error| error.to_string())?;
        rows.map(|row| {
            serde_json::from_str(&row.map_err(|error| error.to_string())?)
                .map_err(|error| error.to_string())
        })
        .collect()
    }

    /// Host-only mutations carry both a revision and a durable command identity.
    pub fn apply(
        &self,
        command_id: &str,
        id: &str,
        expected_revision: u64,
        command: IssueAssignmentCommand,
        now: u64,
    ) -> Result<IssueAssignment, String> {
        let fingerprint = fingerprint(&(id, expected_revision, &command))?;
        let mut connection = self
            .connection
            .lock()
            .map_err(|_| "Issue assignment store lock poisoned")?;
        let tx = connection
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(|error| error.to_string())?;
        if let Some(value) = replay(&tx, command_id, &fingerprint)? {
            return serde_json::from_str(&value).map_err(|error| error.to_string());
        }
        let mut assignment = read(&tx, id)?;
        if assignment.revision != expected_revision {
            return Err(format!(
                "Issue assignment changed: expected {expected_revision}, actual {}",
                assignment.revision
            ));
        }
        if assignment.ownership != IssueOwnership::Held
            && !matches!(
                &command,
                IssueAssignmentCommand::Control {
                    action: IssueControl::Release,
                    ..
                } | IssueAssignmentCommand::BeginSync { .. }
                    | IssueAssignmentCommand::Release
                    | IssueAssignmentCommand::Transfer { .. }
                    | IssueAssignmentCommand::ClaimPrepared { .. }
                    | IssueAssignmentCommand::FreezeAgent { .. }
                    | IssueAssignmentCommand::RecordThread { .. }
                    | IssueAssignmentCommand::RecordBranch { .. }
                    | IssueAssignmentCommand::RecordSync { .. }
                    | IssueAssignmentCommand::SyncFailed { .. }
            )
        {
            return Err("Issue assignment is no longer held".into());
        }
        match command {
            IssueAssignmentCommand::ExecutionFailed { detail } => {
                require_stopped(&assignment)?;
                assignment.execution_error = Some(detail);
                assignment.desired_stage = github::IssueStage::Blocked;
                assignment.sync_state = IssueSyncState::Pending;
            }
            IssueAssignmentCommand::Control { epoch, action } => {
                if matches!(
                    assignment.ownership,
                    IssueOwnership::Completed
                        | IssueOwnership::Released
                        | IssueOwnership::Unclaimed
                ) {
                    return Err("This Issue assignment has no active ownership to control".into());
                }
                if epoch != assignment.epoch {
                    return Err("Issue execution was superseded".into());
                }
                if let IssueControl::Transfer(owner) = &action {
                    if owner.is_empty() || owner.chars().any(char::is_whitespace) {
                        return Err("Choose a responsible GitHub account".into());
                    }
                }
                assignment.epoch = assignment
                    .epoch
                    .checked_add(1)
                    .ok_or("Issue epoch exhausted")?;
                assignment.lease_until = None;
                assignment.auto_start = false;
                assignment.paused = matches!(action, IssueControl::Pause);
                assignment.desired_stage = match &action {
                    IssueControl::Release => github::IssueStage::Todo,
                    IssueControl::Transfer(_) => github::IssueStage::Queued,
                    _ => github::IssueStage::Blocked,
                };
                assignment.ownership = match action {
                    IssueControl::Pause => IssueOwnership::Held,
                    IssueControl::Release => IssueOwnership::Releasing,
                    IssueControl::Cancel => IssueOwnership::Cancelled,
                    IssueControl::Transfer(owner) => {
                        assignment.pending_owner = Some(owner);
                        IssueOwnership::Transferring
                    }
                };
                assignment.sync_state = IssueSyncState::Pending;
            }
            IssueAssignmentCommand::FreezeAgent { role, tools } => {
                if assignment
                    .agent_role
                    .as_ref()
                    .is_some_and(|previous| previous != &role || assignment.agent_tools != tools)
                {
                    return Err("Agent definition is already frozen for this assignment".into());
                }
                assignment.agent_role = Some(role);
                assignment.agent_tools = tools;
            }
            IssueAssignmentCommand::BeginSync { epoch, stage } => {
                if assignment.epoch != epoch {
                    return Err("Issue synchronization was superseded".into());
                }
                if !assignment.attempted_stages.contains(&stage) {
                    assignment.attempted_stages.push(stage);
                }
                assignment.sync_state = IssueSyncState::Pending;
            }
            IssueAssignmentCommand::PrepareSync { stage } => {
                assignment.desired_stage = stage;
                assignment.sync_state = IssueSyncState::Pending;
            }
            IssueAssignmentCommand::FinishExecution { epoch } => {
                if assignment.epoch != epoch {
                    return Err("Issue execution was superseded".into());
                }
                assignment.epoch = assignment
                    .epoch
                    .checked_add(1)
                    .ok_or("Issue epoch exhausted")?;
                assignment.lease_until = None;
                assignment.auto_start = false;
                assignment.paused = false;
                assignment.desired_stage = github::IssueStage::Review;
                assignment.sync_state = IssueSyncState::Pending;
            }
            IssueAssignmentCommand::ClaimPrepared {
                workflow,
                config_revision,
            } => {
                if assignment.ownership != IssueOwnership::Unclaimed || workflow.assignee.is_empty()
                {
                    return Err(
                        "Only an unclaimed branch with a configured assignee can be claimed".into(),
                    );
                }
                workflow.validate()?;
                for issue in &assignment.item.issues {
                    let owner: Option<String> = tx
                        .query_row(
                            "SELECT assignment FROM issue_claims WHERE repository=?1 AND issue=?2",
                            (assignment.repository.key(), &issue.node_id),
                            |row| row.get(0),
                        )
                        .optional()
                        .map_err(|error| error.to_string())?;
                    if owner.is_some() {
                        return Err(format!("Issue #{} is already claimed", issue.number));
                    }
                }
                for issue in &assignment.item.issues {
                    tx.execute(
                        "INSERT INTO issue_claims(repository,issue,assignment) VALUES(?1,?2,?3)",
                        (assignment.repository.key(), &issue.node_id, &assignment.id),
                    )
                    .map_err(|error| error.to_string())?;
                }
                assignment.owner = workflow.assignee.clone();
                assignment.workflow = workflow;
                assignment.config_revision = config_revision;
                assignment.ownership = IssueOwnership::Held;
                assignment.sync_state = IssueSyncState::Pending;
            }

            IssueAssignmentCommand::RecordDelivery { receipt } => {
                require_stopped(&assignment)?;
                if assignment.delivery.as_ref().is_some_and(|existing| {
                    existing.pull_request_number.is_some() && existing.commit != receipt.commit
                }) {
                    return Err("An existing PR binds a different delivery commit".into());
                }
                assignment.delivery = Some(receipt);
            }
            IssueAssignmentCommand::RecordThread { thread_id } => {
                if assignment
                    .thread_id
                    .as_ref()
                    .is_some_and(|old| old != &thread_id)
                {
                    return Err("Assignment already belongs to another Thread".into());
                }
                assignment.thread_id = Some(thread_id);
            }
            IssueAssignmentCommand::Queue => {
                require_stopped(&assignment)?;
                if assignment
                    .delivery
                    .as_ref()
                    .is_some_and(|receipt| receipt.pull_request_number.is_some())
                {
                    return Err(
                        "Resolve the existing PR before starting another implementation".into(),
                    );
                }
                assignment.delivery = None;
                assignment.execution_error = None;
                assignment.auto_start = true;
                assignment.paused = false;
                assignment.desired_stage = github::IssueStage::Queued;
                assignment.sync_state = IssueSyncState::Pending;
            }
            IssueAssignmentCommand::Acquire { epoch, lease_until } => {
                if assignment.lease_until.is_some()
                    || !assignment.auto_start
                    || assignment.sync_state != IssueSyncState::Synced
                    || epoch != assignment.epoch
                    || lease_until <= now
                    || lease_until > now.saturating_add(300)
                {
                    return Err("Issue execution is already reserved or no longer queued".into());
                }
                assignment.lease_until = Some(lease_until);
            }
            IssueAssignmentCommand::Renew { epoch, lease_until } => {
                if epoch != assignment.epoch
                    || lease_until <= now
                    || lease_until > now.saturating_add(300)
                {
                    return Err("Invalid issue execution epoch or lease interval".into());
                }
                if assignment
                    .lease_until
                    .is_some_and(|deadline| now >= deadline)
                {
                    return Err("Expired execution must be stopped before it can resume".into());
                }
                assignment.lease_until = Some(lease_until);
            }
            IssueAssignmentCommand::Stop { epoch } => {
                if epoch != assignment.epoch {
                    return Err("Issue execution was superseded".into());
                }
                assignment.epoch = assignment
                    .epoch
                    .checked_add(1)
                    .ok_or("Issue execution epoch exhausted")?;
                assignment.lease_until = None;
                assignment.auto_start = false;
                assignment.paused = true;
                assignment.desired_stage = github::IssueStage::Blocked;
                assignment.sync_state = IssueSyncState::Pending;
            }
            IssueAssignmentCommand::Transfer { owner } => {
                require_stopped(&assignment)?;
                let synchronized = assignment.ownership == IssueOwnership::Transferring
                    && assignment.sync_state == IssueSyncState::Synced;
                if owner.is_empty() || owner.chars().any(char::is_whitespace) {
                    return Err("A responsible account is required".into());
                }
                assignment.owner = owner;
                assignment.pending_owner = None;
                assignment.ownership = IssueOwnership::Held;
                assignment.delivery = None;
                assignment.desired_stage = github::IssueStage::Queued;
                assignment.paused = true;
                assignment.epoch = assignment
                    .epoch
                    .checked_add(1)
                    .ok_or("Issue execution epoch exhausted")?;
                assignment.sync_state = if synchronized {
                    IssueSyncState::Synced
                } else {
                    IssueSyncState::Pending
                };
            }
            IssueAssignmentCommand::Release
            | IssueAssignmentCommand::Complete
            | IssueAssignmentCommand::Cancel => {
                require_stopped(&assignment)?;
                let synchronized = assignment.ownership == IssueOwnership::Releasing
                    && assignment.sync_state == IssueSyncState::Synced
                    || matches!(command, IssueAssignmentCommand::Complete);
                assignment.ownership = match command {
                    IssueAssignmentCommand::Release => IssueOwnership::Released,
                    IssueAssignmentCommand::Complete => IssueOwnership::Completed,
                    _ => IssueOwnership::Cancelled,
                };
                if assignment.ownership != IssueOwnership::Cancelled {
                    tx.execute("DELETE FROM issue_claims WHERE assignment=?1", [id])
                        .map_err(|error| error.to_string())?;
                }
                assignment.desired_stage = match assignment.ownership {
                    IssueOwnership::Released => github::IssueStage::Todo,
                    IssueOwnership::Completed => github::IssueStage::Completed,
                    _ => github::IssueStage::Blocked,
                };
                assignment.sync_state = if synchronized {
                    IssueSyncState::Synced
                } else {
                    IssueSyncState::Pending
                };
            }
            IssueAssignmentCommand::RecordBranch { linked_branch_id } => {
                if assignment
                    .linked_branch_id
                    .as_ref()
                    .is_some_and(|id| id != &linked_branch_id)
                {
                    return Err("Assignment is already linked to a different branch".into());
                }
                assignment.linked_branch_id = Some(linked_branch_id);
            }
            IssueAssignmentCommand::RecordSync { stage, labels } => {
                assignment.synced_stage = Some(stage);
                assignment.synced_labels = labels;
                if assignment.desired_stage == stage {
                    assignment.sync_state = IssueSyncState::Synced;
                    assignment.attempted_stages.clear();
                    assignment.detail.clear();
                } else {
                    assignment.sync_state = IssueSyncState::Pending;
                }
            }
            IssueAssignmentCommand::SyncFailed { state, detail } => {
                if state == IssueSyncState::Synced {
                    return Err("A failed synchronization cannot be marked synced".into());
                }
                assignment.sync_state = state;
                assignment.detail = detail;
            }
        }
        assignment.revision = assignment
            .revision
            .checked_add(1)
            .ok_or("Issue assignment revision exhausted")?;
        assignment.updated_at = now;
        tx.execute(
            "UPDATE issue_assignments SET revision=?1,value=?2 WHERE id=?3",
            (
                i64::try_from(assignment.revision).map_err(|_| "Issue revision out of range")?,
                serde_json::to_string(&assignment).map_err(|error| error.to_string())?,
                id,
            ),
        )
        .map_err(|error| error.to_string())?;
        record(&tx, command_id, &fingerprint, &assignment)?;
        tx.commit().map_err(|error| error.to_string())?;
        Ok(assignment)
    }
}

fn require_stopped(assignment: &IssueAssignment) -> Result<(), String> {
    if assignment.lease_until.is_some() {
        return Err("Stop the previous execution before changing its ownership".into());
    }
    Ok(())
}
fn read(connection: &Connection, id: &str) -> Result<IssueAssignment, String> {
    let value: String = connection
        .query_row(
            "SELECT value FROM issue_assignments WHERE id=?1",
            [id],
            |row| row.get(0),
        )
        .map_err(|error| error.to_string())?;
    serde_json::from_str(&value)
        .map_err(|error| format!("Invalid stored issue assignment: {error}"))
}
fn replay(connection: &Connection, id: &str, fingerprint: &str) -> Result<Option<String>, String> {
    let row: Option<(String, String)> = connection
        .query_row(
            "SELECT fingerprint,value FROM issue_assignment_receipts WHERE id=?1",
            [id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()
        .map_err(|error| error.to_string())?;
    match row {
        Some((stored, value)) if stored == fingerprint => Ok(Some(value)),
        Some(_) => Err("Issue command conflicts with a previous request".into()),
        None => Ok(None),
    }
}
fn record(
    connection: &Connection,
    id: &str,
    fingerprint: &str,
    result: &impl Serialize,
) -> Result<(), String> {
    connection
        .execute(
            "INSERT INTO issue_assignment_receipts(id,fingerprint,value) VALUES(?1,?2,?3)",
            (
                id,
                fingerprint,
                serde_json::to_string(result).map_err(|error| error.to_string())?,
            ),
        )
        .map_err(|error| error.to_string())?;
    Ok(())
}
fn fingerprint(value: &impl Serialize) -> Result<String, String> {
    Ok(fingerprint_text(
        &serde_json::to_string(value).map_err(|error| error.to_string())?,
    ))
}
fn fingerprint_text(value: &str) -> String {
    format!("{:x}", Sha256::digest(value.as_bytes()))
}

#[cfg(test)]
#[path = "issue_assignments_tests.rs"]
mod tests;

#[derive(Clone, Copy, Eq, PartialEq, Serialize)]
enum Reservation {
    Branch,
    Assignment,
    Execute,
}
