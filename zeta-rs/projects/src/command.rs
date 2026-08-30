use crate::ProjectRoot;
use serde::Deserialize;
use serde::Serialize;
use zeta_file_access::DirId;
use zeta_protocol::CommandId;
use zeta_protocol::ProjectId;
use zeta_protocol::SessionId;
use zeta_protocol::WorkRunId;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum ProjectCommand {
    Create {
        name: String,
        description: String,
    },
    UpdateDetails {
        name: String,
        description: String,
    },
    AddRoot {
        root: ProjectRoot,
    },
    UpdateRootDetails {
        dir_id: DirId,
        name: String,
        purpose: String,
    },
    RemoveRoot {
        dir_id: DirId,
    },
    LinkSession {
        session_id: SessionId,
    },
    UnlinkSession {
        session_id: SessionId,
    },
    LinkWorkRun {
        work_run_id: WorkRunId,
    },
    UnlinkWorkRun {
        work_run_id: WorkRunId,
    },
    Archive,
    Restore,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectCommandRequest {
    pub command_id: CommandId,
    pub project_id: ProjectId,
    pub expected_revision: u64,
    pub command: ProjectCommand,
}
