use super::RemainingContextWindow;
use super::StatusViewData;
use super::status_pane_spec;
use crate::components::detail_list::DetailList;
use crate::components::pane::PaneSpec;
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
}

pub(crate) fn load_status_pane_spec<T>(
    client: &mut AppServerClient<T>,
    scope: StatusRequestScope<'_>,
) -> Result<PaneSpec<DetailList>, ClientError>
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
    let model = thread.turns.last().and_then(|turn| turn.model.as_ref());
    let models = client.list_models()?;
    let model_entry =
        model.and_then(|model| models.models.iter().find(|entry| &entry.model == model));
    let available = model_entry
        .and_then(|entry| entry.available_context_window)
        .map(u64::from);
    let remaining = remaining_context_window(available, model, &thread);
    let model = model
        .map(|model| format!("{}/{}", model.provider, model.model))
        .unwrap_or_else(|| "not configured".into());

    Ok(status_pane_spec(StatusViewData {
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
        return RemainingContextWindow::Exact {
            remaining_tokens: available,
            available_tokens: available,
        };
    };
    if latest_turn.model.as_ref() != model {
        return RemainingContextWindow::Unknown;
    }
    let Some(usage) = latest_turn.context_usage.as_ref() else {
        return RemainingContextWindow::Unknown;
    };
    let remaining = available.saturating_sub(usage.used_tokens);
    match usage.source {
        ModelContextUsageSource::ProviderReported => RemainingContextWindow::Exact {
            remaining_tokens: remaining,
            available_tokens: available,
        },
        ModelContextUsageSource::Estimated => RemainingContextWindow::Estimated {
            remaining_tokens: remaining,
            available_tokens: available,
        },
    }
}

#[cfg(test)]
#[path = "request_tests.rs"]
mod tests;
