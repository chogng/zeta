use super::{AppServer, ConnectionState, RpcError, decode, result};
use serde_json::Value;
use zeta_app_server_protocol::protocol::error::AppServerErrorName;
use zeta_app_server_protocol::protocol::search::WorkspaceSearchCancelParams;
use zeta_app_server_protocol::protocol::search::WorkspaceSearchCaseSensitivity as WorkspaceSearchProtocolCaseSensitivity;
use zeta_app_server_protocol::protocol::search::WorkspaceSearchMatch as WorkspaceSearchProtocolMatch;
use zeta_app_server_protocol::protocol::search::WorkspaceSearchMatchRange as WorkspaceSearchProtocolMatchRange;
use zeta_app_server_protocol::protocol::search::WorkspaceSearchPatternKind;
use zeta_app_server_protocol::protocol::search::WorkspaceSearchReadParams;
use zeta_app_server_protocol::protocol::search::WorkspaceSearchReadResult;
use zeta_app_server_protocol::protocol::search::WorkspaceSearchStartParams;
use zeta_app_server_protocol::protocol::search::WorkspaceSearchStartResult;
use zeta_workspace_search::WorkspaceSearchCaseSensitivity;
use zeta_workspace_search::WorkspaceSearchError;
use zeta_workspace_search::WorkspaceSearchOwner;
use zeta_workspace_search::WorkspaceSearchPage;
use zeta_workspace_search::WorkspaceSearchPattern;
use zeta_workspace_search::WorkspaceSearchQuery;

impl AppServer {
    pub(super) fn workspace_search_start(
        &self,
        connection: &ConnectionState,
        params: &Value,
    ) -> Result<Value, RpcError> {
        let params: WorkspaceSearchStartParams = decode(params)?;
        let workspace_folder_id = params.workspace_folder_id.clone();
        let search = self.search_service_for_request(
            workspace_folder_id.as_deref(),
            params.session_directory.as_ref(),
        )?;
        let search_id = search
            .start(search_owner(connection), search_query(params))
            .map_err(search_error)?;
        result(&WorkspaceSearchStartResult { search_id })
    }

    pub(super) fn workspace_search_read(
        &self,
        connection: &ConnectionState,
        params: &Value,
    ) -> Result<Value, RpcError> {
        let params: WorkspaceSearchReadParams = decode(params)?;
        let workspace_folder_id = params.workspace_folder_id;
        let search = self.search_service_for_request(
            workspace_folder_id.as_deref(),
            params.session_directory.as_ref(),
        )?;
        let search_id = params.search_id;
        let page = search
            .read(
                search_owner(connection),
                &search_id,
                params.after_match,
                params.max_matches,
            )
            .map_err(search_error)?;
        result(&search_page(search_id, page))
    }

    pub(super) fn workspace_search_cancel(
        &self,
        connection: &ConnectionState,
        params: &Value,
    ) -> Result<Value, RpcError> {
        let params: WorkspaceSearchCancelParams = decode(params)?;
        self.search_service_for_request(
            params.workspace_folder_id.as_deref(),
            params.session_directory.as_ref(),
        )?
        .cancel(search_owner(connection), &params.search_id)
        .map_err(search_error)?;
        result(&())
    }

    fn search_service_for_request(
        &self,
        workspace_folder_id: Option<&str>,
        session_directory: Option<
            &zeta_app_server_protocol::protocol::workspace::WorkspaceSessionDirectorySelector,
        >,
    ) -> Result<std::sync::Arc<zeta_workspace_search::WorkspaceSearchService>, RpcError> {
        match (workspace_folder_id, session_directory) {
            (Some(_), Some(_)) => Err(RpcError::new(-32602, AppServerErrorName::InvalidParams)),
            (_, Some(selector)) => self.workspace_search_service_for_session_directory(selector),
            (_, None) => self.workspace_search_service_for(workspace_folder_id),
        }
    }
}

fn search_owner(connection: &ConnectionState) -> WorkspaceSearchOwner {
    WorkspaceSearchOwner::new(connection.connection_id)
}

fn search_query(params: WorkspaceSearchStartParams) -> WorkspaceSearchQuery {
    WorkspaceSearchQuery {
        query: params.query,
        pattern: match params.pattern_kind {
            WorkspaceSearchPatternKind::Literal => WorkspaceSearchPattern::Literal,
            WorkspaceSearchPatternKind::Regex => WorkspaceSearchPattern::Regex,
        },
        case_sensitivity: match params.case_sensitivity {
            WorkspaceSearchProtocolCaseSensitivity::Smart => WorkspaceSearchCaseSensitivity::Smart,
            WorkspaceSearchProtocolCaseSensitivity::Sensitive => {
                WorkspaceSearchCaseSensitivity::Sensitive
            }
            WorkspaceSearchProtocolCaseSensitivity::Insensitive => {
                WorkspaceSearchCaseSensitivity::Insensitive
            }
        },
        include_patterns: params.include_patterns,
        exclude_patterns: params.exclude_patterns,
        max_results: params.max_results,
    }
}

fn search_page(search_id: String, page: WorkspaceSearchPage) -> WorkspaceSearchReadResult {
    WorkspaceSearchReadResult {
        search_id,
        matches: page
            .matches
            .into_iter()
            .map(|search_match| WorkspaceSearchProtocolMatch {
                path: search_match.path,
                line_number: search_match.line_number,
                preview: search_match.preview,
                ranges: search_match
                    .ranges
                    .into_iter()
                    .map(|range| WorkspaceSearchProtocolMatchRange {
                        start: range.start,
                        end: range.end,
                    })
                    .collect(),
            })
            .collect(),
        next_match: page.next_match,
        completed: page.completed,
        limit_hit: page.limit_hit,
        error: page.error,
    }
}

fn search_error(error: WorkspaceSearchError) -> RpcError {
    match error {
        WorkspaceSearchError::InvalidInput => {
            RpcError::new(-32602, AppServerErrorName::InvalidParams)
        }
        WorkspaceSearchError::NotFound => RpcError::new(-32051, AppServerErrorName::SearchNotFound),
        WorkspaceSearchError::NotOwner => RpcError::new(-32052, AppServerErrorName::SearchNotOwner),
        WorkspaceSearchError::Busy => RpcError::new(-32053, AppServerErrorName::SearchBusy),
        WorkspaceSearchError::Unavailable => {
            RpcError::new(-32050, AppServerErrorName::SearchUnavailable)
        }
    }
}

#[cfg(test)]
#[path = "search_operations_tests.rs"]
mod tests;
