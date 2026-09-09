use crate::SqliteDurability;
use crate::open_sqlite_database;
use github::IssueTask;
use rusqlite::Connection;
use rusqlite::OptionalExtension;
use std::path::Path;
use std::sync::Mutex;

/// Profile database owner of durable issue-task associations.
pub struct SqliteIssueTaskStore {
    connection: Mutex<Connection>,
}

impl SqliteIssueTaskStore {
    pub fn record_pull_request(
        &self,
        session_id: &str,
        pull_request: &github::PullRequest,
    ) -> Result<(), String> {
        let mut connection = self
            .connection
            .lock()
            .map_err(|_| "Issue task store lock poisoned")?;
        let transaction = connection
            .transaction()
            .map_err(|error| error.to_string())?;
        let value: String = transaction
            .query_row(
                "SELECT task FROM issue_tasks WHERE session_id = ?1",
                [session_id],
                |row| row.get(0),
            )
            .optional()
            .map_err(|error| error.to_string())?
            .ok_or_else(|| "This Session has no associated issues".to_string())?;
        let mut task: IssueTask =
            serde_json::from_str(&value).map_err(|error| error.to_string())?;
        if task
            .pull_request
            .as_ref()
            .is_some_and(|existing| existing.node_id != pull_request.node_id)
        {
            return Err("Task is already associated with another PR".into());
        }
        if pull_request.head.name != task.branch || pull_request.base.name != task.target_branch {
            return Err("PR branches do not match this task".into());
        }
        task.pull_request = Some(pull_request.clone());
        let value = serde_json::to_string(&task).map_err(|error| error.to_string())?;
        transaction
            .execute(
                "UPDATE issue_tasks SET task = ?1 WHERE session_id = ?2",
                (&value, session_id),
            )
            .map_err(|error| error.to_string())?;
        transaction.commit().map_err(|error| error.to_string())
    }
    pub fn open(path: &Path) -> Result<Self, String> {
        let connection = open_sqlite_database(path, SqliteDurability::Durable)
            .map_err(|error| error.to_string())?;
        connection.execute_batch("CREATE TABLE IF NOT EXISTS issue_tasks (command_id TEXT PRIMARY KEY, session_id TEXT NOT NULL UNIQUE, fingerprint TEXT NOT NULL, task TEXT NOT NULL)").map_err(|error| error.to_string())?;
        Ok(Self {
            connection: Mutex::new(connection),
        })
    }

    pub fn read_command(&self, command_id: &str) -> Result<Option<IssueTask>, String> {
        self.read(
            "SELECT task FROM issue_tasks WHERE command_id = ?1",
            command_id,
        )
    }

    pub fn read_session(&self, session_id: &str) -> Result<Option<IssueTask>, String> {
        self.read(
            "SELECT task FROM issue_tasks WHERE session_id = ?1",
            session_id,
        )
    }

    fn read(&self, query: &str, key: &str) -> Result<Option<IssueTask>, String> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| "Issue task store lock poisoned")?;
        let value: Option<String> = connection
            .query_row(query, [key], |row| row.get(0))
            .optional()
            .map_err(|error| error.to_string())?;
        value
            .map(|value| {
                serde_json::from_str(&value)
                    .map_err(|error| format!("Invalid stored issue task: {error}"))
            })
            .transpose()
    }

    /// Stores the immutable preparation before provisioning a Thread; identical retries reuse it.
    pub fn prepare(&self, task: &IssueTask) -> Result<(), String> {
        if task.command_id.is_empty() || task.session_id.is_empty() || task.issues.is_empty() {
            return Err("Issue task requires command, Session and issue identities".into());
        }
        let value = serde_json::to_string(task).map_err(|error| error.to_string())?;
        let connection = self
            .connection
            .lock()
            .map_err(|_| "Issue task store lock poisoned")?;
        connection.execute("INSERT INTO issue_tasks (command_id, session_id, fingerprint, task) VALUES (?1, ?2, ?3, ?4) ON CONFLICT(command_id) DO NOTHING", (&task.command_id, &task.session_id, &task.fingerprint, &value)).map_err(|error| error.to_string())?;
        let stored: String = connection
            .query_row(
                "SELECT task FROM issue_tasks WHERE command_id = ?1",
                [&task.command_id],
                |row| row.get(0),
            )
            .map_err(|error| error.to_string())?;
        if stored != value {
            return Err("Issue task command conflicts with an existing preparation".into());
        }
        Ok(())
    }
}
