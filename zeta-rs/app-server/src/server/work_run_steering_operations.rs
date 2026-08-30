use super::AppServer;
use super::ConnectionState;
use super::RpcError;
use super::decode;
use serde_json::Value;
use zeta_app_server_protocol::protocol::work_runs::WorkRunAttemptScopeExpansionRequestParams;
use zeta_app_server_protocol::protocol::work_runs::WorkRunConflictRecordParams;
use zeta_app_server_protocol::protocol::work_runs::WorkRunConflictResolveParams;
use zeta_app_server_protocol::protocol::work_runs::WorkRunDecisionRecordParams;
use zeta_work_coordination::WorkRunCommand;
use zeta_work_coordination::WorkRunCommandRequest;

impl AppServer {
    pub(super) fn work_run_decision_record(
        &self,
        connection: &ConnectionState,
        params: &Value,
    ) -> Result<Value, RpcError> {
        let params: WorkRunDecisionRecordParams = decode(params)?;
        self.apply_work_run(
            connection,
            WorkRunCommandRequest {
                command_id: params.command_id,
                work_run_id: params.work_run_id,
                expected_revision: params.expected_revision,
                command: WorkRunCommand::RecordDecision {
                    decision_id: params.decision_id,
                    authority: params.authority,
                    scope: params.scope,
                    statement: params.statement,
                },
            },
        )
    }

    pub(super) fn work_run_attempt_scope_expansion_request(
        &self,
        connection: &ConnectionState,
        params: &Value,
    ) -> Result<Value, RpcError> {
        let params: WorkRunAttemptScopeExpansionRequestParams = decode(params)?;
        self.apply_work_run(
            connection,
            WorkRunCommandRequest {
                command_id: params.command_id,
                work_run_id: params.work_run_id,
                expected_revision: params.expected_revision,
                command: WorkRunCommand::RequestScopeExpansion {
                    attempt_id: params.attempt_id,
                    evidence: params.evidence,
                },
            },
        )
    }

    pub(super) fn work_run_conflict_record(
        &self,
        connection: &ConnectionState,
        params: &Value,
    ) -> Result<Value, RpcError> {
        let params: WorkRunConflictRecordParams = decode(params)?;
        self.apply_work_run(
            connection,
            WorkRunCommandRequest {
                command_id: params.command_id,
                work_run_id: params.work_run_id,
                expected_revision: params.expected_revision,
                command: WorkRunCommand::RecordConflict {
                    conflict_id: params.conflict_id,
                    attempt_ids: params.attempt_ids,
                    resource: params.resource,
                    evidence: params.evidence,
                },
            },
        )
    }

    pub(super) fn work_run_conflict_resolve(
        &self,
        connection: &ConnectionState,
        params: &Value,
    ) -> Result<Value, RpcError> {
        let params: WorkRunConflictResolveParams = decode(params)?;
        self.apply_work_run(
            connection,
            WorkRunCommandRequest {
                command_id: params.command_id,
                work_run_id: params.work_run_id,
                expected_revision: params.expected_revision,
                command: WorkRunCommand::ResolveConflict {
                    conflict_id: params.conflict_id,
                    decision_id: params.decision_id,
                },
            },
        )
    }
}
