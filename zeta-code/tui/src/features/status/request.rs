use super::RemainingContextWindow;
use super::StatusViewData;
use super::status_view;
use crate::components::pane::PaneViewModel;
use crate::components::selection::SelectionViewModel;
use zeta_app_server_client::AppServerClient;
use zeta_app_server_client::ClientError;
use zeta_app_server_client::JsonRpcTransport;
use zeta_app_server_protocol::protocol::session::SessionThreadReadParams;
use zeta_protocol::ModelContextUsageSource;
use zeta_protocol::ModelRef;
use zeta_protocol::SessionId;
use zeta_protocol::ThreadId;

pub(crate) struct StatusRequestScope<'a> {
    pub(crate) session_id: &'a SessionId,
    pub(crate) thread_id: &'a ThreadId,
    pub(crate) model: Option<&'a ModelRef>,
}

pub(crate) fn load_status_view<T>(
    client: &mut AppServerClient<T>,
    scope: StatusRequestScope<'_>,
) -> Result<PaneViewModel<SelectionViewModel>, ClientError>
where
    T: JsonRpcTransport,
{
    let thread = client
        .read_session_thread(SessionThreadReadParams {
            session_id: scope.session_id.clone(),
            thread_id: scope.thread_id.clone(),
            history: None,
        })?
        .thread;
    let models = client.list_models()?;
    let model_entry = scope
        .model
        .and_then(|model| models.models.iter().find(|entry| &entry.model == model));
    let available = model_entry
        .and_then(|entry| entry.available_context_window)
        .map(u64::from);
    let remaining = remaining_context_window(available, scope.model, &thread);
    let model = scope
        .model
        .map(|model| format!("{}/{}", model.provider, model.model))
        .unwrap_or_else(|| "not configured".into());

    Ok(status_view(StatusViewData {
        model: &model,
        full_context_window: model_entry
            .and_then(|entry| entry.context_window)
            .map(u64::from),
        available_context_window: available,
        remaining_context_window: remaining,
        session_id: scope.session_id.as_str(),
        thread_id: scope.thread_id.as_str(),
        thread_sequence: thread.sequence,
    }))
}

fn remaining_context_window(
    available: Option<u64>,
    model: Option<&ModelRef>,
    thread: &zeta_protocol::Thread,
) -> RemainingContextWindow {
    let Some(available) = available else {
        return RemainingContextWindow::Unknown;
    };
    let Some(latest_turn) = thread.turns.last() else {
        return RemainingContextWindow::Exact(available);
    };
    if latest_turn.model.as_ref() != model {
        return RemainingContextWindow::Unknown;
    }
    let Some(usage) = latest_turn.context_usage.as_ref() else {
        return RemainingContextWindow::Unknown;
    };
    let remaining = available.saturating_sub(usage.used_tokens);
    match usage.source {
        ModelContextUsageSource::ProviderReported => RemainingContextWindow::Exact(remaining),
        ModelContextUsageSource::Estimated => RemainingContextWindow::Estimated(remaining),
    }
}

#[cfg(test)]
#[path = "request_tests.rs"]
mod tests;
