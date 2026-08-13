use super::update_broker::UpdateBroker;
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::Duration;
use zeta_plugins::PluginActivationAuthority;
use zeta_plugins::PluginProfileRequest;
use zeta_plugins::PluginProfileRequestEnablement;
use zeta_skills_extension::SkillCatalogReload;
use zeta_skills_extension::SkillRuntime;

pub(super) struct PluginWatcher {
    shutdown: Option<std::sync::mpsc::Sender<()>>,
    thread: Option<JoinHandle<()>>,
}

impl PluginWatcher {
    pub(super) fn start(
        authority: &PluginActivationAuthority,
        updates: Arc<UpdateBroker>,
        skills: Option<Arc<SkillRuntime>>,
    ) -> Self {
        let changes = authority.subscribe();
        let (shutdown, shutdown_receiver) = std::sync::mpsc::channel();
        let thread = std::thread::Builder::new()
            .name("zeta-plugin-notifications".into())
            .spawn(move || {
                loop {
                    if shutdown_receiver.try_recv().is_ok() {
                        break;
                    }
                    match changes.recv_timeout(Duration::from_millis(100)) {
                        Ok(change) => {
                            updates.publish_plugins_changed(
                                change.revision,
                                change.activation_generation,
                            );
                            if let Some(skills) = &skills
                                && let Err(error) = skills.list(SkillCatalogReload::Refresh)
                            {
                                log::error!("failed to reconcile Plugin Skill sources: {error}");
                            }
                        }
                        Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {}
                        Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
                    }
                }
            })
            .ok();
        Self {
            shutdown: Some(shutdown),
            thread,
        }
    }
}

impl Drop for PluginWatcher {
    fn drop(&mut self) {
        if let Some(shutdown) = self.shutdown.take() {
            let _ = shutdown.send(());
        }
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

pub(super) struct PluginProfileWatcher {
    shutdown: Option<std::sync::mpsc::Sender<()>>,
    thread: Option<JoinHandle<()>>,
}

impl PluginProfileWatcher {
    pub(super) fn start(
        config: Arc<zeta_config::ConfigStore>,
        marketplaces: zeta_plugins::PluginMarketplaceService,
    ) -> Self {
        let changes = config.subscribe_changes();
        let (shutdown, shutdown_receiver) = std::sync::mpsc::channel();
        let thread = std::thread::Builder::new()
            .name("zeta-plugin-profile".into())
            .spawn(move || {
                loop {
                    if shutdown_receiver.try_recv().is_ok() {
                        break;
                    }
                    match changes.recv_timeout(Duration::from_millis(100)) {
                        Ok(_) => {
                            while changes.try_recv().is_ok() {}
                            if let Err(error) = reconcile_profile(&config, &marketplaces) {
                                log::error!("failed to reconcile Plugin profile requests: {error}");
                            }
                        }
                        Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {}
                        Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
                    }
                }
            })
            .ok();
        Self {
            shutdown: Some(shutdown),
            thread,
        }
    }
}

impl Drop for PluginProfileWatcher {
    fn drop(&mut self) {
        if let Some(shutdown) = self.shutdown.take() {
            let _ = shutdown.send(());
        }
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

fn reconcile_profile(
    config: &zeta_config::ConfigStore,
    marketplaces: &zeta_plugins::PluginMarketplaceService,
) -> Result<(), String> {
    let snapshot = config.read_snapshot().map_err(|error| error.to_string())?;
    let requests = snapshot
        .values
        .plugins
        .requests
        .values()
        .map(|request| {
            Ok(PluginProfileRequest {
                id: zeta_plugins::PluginId::new(request.plugin_id.as_str())
                    .map_err(|error| error.to_string())?,
                version: zeta_plugins::PluginVersion::new(request.version.as_str())
                    .map_err(|error| error.to_string())?,
                enablement: match request.enablement {
                    zeta_config::PluginRequestEnablement::Disabled => {
                        PluginProfileRequestEnablement::Disabled
                    }
                    zeta_config::PluginRequestEnablement::Enabled => {
                        PluginProfileRequestEnablement::Enabled
                    }
                },
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    marketplaces
        .reconcile_profile(requests)
        .map(|_| ())
        .map_err(|error| error.to_string())
}
