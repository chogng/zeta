use std::sync::Arc;
use zeta_extension_api::ExtensionError;
use zeta_extension_api::ThreadContext;
use zeta_extension_api::ThreadLifecycle;

pub(super) struct UsageObserver {
    pub analytics: Arc<analytics::Analytics>,
    pub config: Arc<zeta_config::ConfigStore>,
}
impl zeta_extension_api::LifecycleObserver for UsageObserver {
    fn thread_changed(&self, _: ThreadContext<'_>, event: &ThreadLifecycle) {
        if matches!(event, ThreadLifecycle::TurnStarted(_)) {
            self.analytics.record(analytics::UsageEvent::TurnStarted);
        }
    }
    fn config_changed(&self, _: u64) {
        match self.config.read_snapshot() {
            Ok(snapshot) => self
                .analytics
                .set_enabled(features::Feature::Analytics.enabled(&snapshot.values.features)),
            Err(error) => log::error!("cannot apply analytics preference: {error}"),
        }
    }
}

pub(super) struct QueueExtension(pub Arc<queue::QueueStore>);
impl zeta_extension_api::IdleContributor for QueueExtension {
    fn contribute(&self, _: ThreadContext<'_>) {
        self.0.wake();
    }
}
impl zeta_extension_api::ItemContributor for QueueExtension {
    fn contribute(
        &self,
        context: ThreadContext<'_>,
    ) -> Result<Vec<extension_items::ExtensionItem>, ExtensionError> {
        self.0
            .list(context.session_id, context.thread_id)
            .map_err(|error| ExtensionError::new(error.to_string()))?
            .into_iter()
            .filter(|message| {
                !matches!(
                    message.status,
                    queue::QueueStatus::Started | queue::QueueStatus::Cancelled
                )
            })
            .map(|message| {
                let status = match message.status {
                    queue::QueueStatus::Pending | queue::QueueStatus::Paused => {
                        extension_items::ExtensionItemStatus::Pending
                    }
                    queue::QueueStatus::Delivering => extension_items::ExtensionItemStatus::Running,
                    queue::QueueStatus::Started => extension_items::ExtensionItemStatus::Completed,
                    queue::QueueStatus::Rejected => extension_items::ExtensionItemStatus::Failed,
                    queue::QueueStatus::Cancelled => {
                        extension_items::ExtensionItemStatus::Cancelled
                    }
                };
                let body = message
                    .request
                    .input
                    .iter()
                    .filter_map(|input| match input {
                        zeta_protocol::UserInput::Text { text } => Some(text.as_str()),
                        _ => None,
                    })
                    .collect::<Vec<_>>()
                    .join("\n");
                // Display has a separate bound; the full durable input remains available in queue/list.
                let body = body.chars().take(16_384).collect();
                Ok(extension_items::ExtensionItem {
                    extension: "queue".into(),
                    id: message.request.command_id.to_string(),
                    title: message.error.unwrap_or_else(|| "Queued message".into()),
                    body,
                    status,
                })
            })
            .collect()
    }
}
