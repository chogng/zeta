use std::sync::Arc;
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
