use std::collections::BTreeMap;
use std::num::NonZeroU64;
use std::sync::Arc;

use zeta_editor_extension_host::CancelReason;
use zeta_editor_extension_host::ExtensionHostError;
use zeta_editor_extension_host::ExtensionHostSupervisor;
use zeta_workspace::TrustedWorkspace;

use super::ExtensionHostRuntimeError;
use super::FleetState;
use super::MAXIMUM_FLEET_EXTENSIONS;
use super::RuntimeEntry;
use super::RuntimeInner;
use super::authority::prepare_extension;
use super::cancel_handles;
use super::nonzero_incarnation;
use super::projection;
use super::projection::ExtensionHostExtensionSnapshot;
use super::projection::ExtensionHostFleetSnapshot;
use super::projection::extension_projection;
use super::projection::runtime_failure;
use super::source;
use super::source::EditorExtensionDeployment;

impl RuntimeInner {
    pub(super) fn reconcile_authority_locked(
        &self,
        force: bool,
    ) -> Result<ExtensionHostFleetSnapshot, ExtensionHostRuntimeError> {
        let workspace = self
            .state
            .lock()
            .map_err(|_| ExtensionHostRuntimeError::Internal)?
            .workspace
            .clone();
        let Some(workspace) = workspace else {
            return Ok(self.snapshot());
        };
        workspace
            .ensure_active()
            .map_err(|_| ExtensionHostRuntimeError::Host(ExtensionHostError::AuthorityDenied))?;
        let source_snapshot = source::combined_deployments(
            self.plugin_authority.as_ref(),
            self.marketplace_manager.as_ref(),
            self.marketplace_admission.as_ref(),
        )?;
        if !force
            && self
                .state
                .lock()
                .map_err(|_| ExtensionHostRuntimeError::Internal)?
                .source_revision
                == source_snapshot.revision
        {
            return Ok(self.snapshot());
        }
        let extension_count = source_snapshot.deployments.len();
        if extension_count > MAXIMUM_FLEET_EXTENSIONS {
            self.retire_current(CancelReason::AuthorityRevoked)?;
            return Err(ExtensionHostRuntimeError::QuotaExceeded);
        }
        let activation_generation = self
            .state
            .lock()
            .map_err(|_| ExtensionHostRuntimeError::Internal)?
            .authority_generation
            .checked_add(1)
            .ok_or(ExtensionHostRuntimeError::Internal)?;
        let generation =
            NonZeroU64::new(activation_generation).ok_or(ExtensionHostRuntimeError::Internal)?;
        self.retire_current(CancelReason::AuthorityRevoked)?;
        let mut entries = BTreeMap::new();
        for deployment in &source_snapshot.deployments {
            let entry = self.build_entry(&workspace, deployment, generation);
            if entries.insert(entry.fallback.id.clone(), entry).is_some() {
                return Err(ExtensionHostRuntimeError::Internal);
            }
        }
        let published = {
            let mut state = self
                .state
                .lock()
                .map_err(|_| ExtensionHostRuntimeError::Internal)?;
            state.entries = entries;
            state.authority_generation = activation_generation;
            state.source_revision = source_snapshot.revision;
            self.refresh_generation_locked(&mut state)?
        };
        self.publish(published);
        Ok(self.snapshot())
    }

    fn build_entry(
        &self,
        workspace: &TrustedWorkspace,
        deployment: &EditorExtensionDeployment,
        generation: NonZeroU64,
    ) -> RuntimeEntry {
        let version = deployment.version.clone();
        let fallback = ExtensionHostExtensionSnapshot {
            id: deployment.id.clone(),
            version: version.clone(),
            package_digest: deployment.package_digest.clone(),
            runtime_api_version: deployment.params.runtime_api_version,
            activation_generation: generation.get(),
            incarnation: None,
            lifecycle: projection::ExtensionHostLifecycle::Failed,
            failure: None,
            stderr: String::new(),
            output_events: Vec::new(),
            registrations: Vec::new(),
        };
        let prepared = prepare_extension(workspace, deployment, generation);
        let supervisor = prepared.and_then(|prepared| {
            ExtensionHostSupervisor::new(
                Arc::clone(&self.launcher),
                prepared.command,
                prepared.activation,
                self.limits.clone(),
                self.restart_policy,
            )
        });
        match supervisor {
            Ok(supervisor) => {
                let failure = supervisor
                    .start()
                    .err()
                    .map(|error| runtime_failure(&error, nonzero_incarnation(&supervisor)));
                RuntimeEntry {
                    version,
                    supervisor: Some(supervisor),
                    fallback,
                    failure,
                }
            }
            Err(error) => RuntimeEntry {
                version,
                supervisor: None,
                fallback,
                failure: Some(runtime_failure(&error, None)),
            },
        }
    }

    pub(super) fn reconcile_health_locked(
        &self,
    ) -> Result<ExtensionHostFleetSnapshot, ExtensionHostRuntimeError> {
        let supervisors = self
            .state
            .lock()
            .map_err(|_| ExtensionHostRuntimeError::Internal)?
            .entries
            .iter()
            .filter_map(|(id, entry)| {
                entry
                    .supervisor
                    .clone()
                    .map(|supervisor| (id.clone(), supervisor))
            })
            .collect::<Vec<_>>();
        let outcomes = supervisors
            .into_iter()
            .map(|(id, supervisor)| {
                let error = supervisor.reconcile().err();
                (id, supervisor, error)
            })
            .collect::<Vec<_>>();
        let published = {
            let mut state = self
                .state
                .lock()
                .map_err(|_| ExtensionHostRuntimeError::Internal)?;
            for (id, supervisor, error) in outcomes {
                let Some(entry) = state.entries.get_mut(&id) else {
                    continue;
                };
                if entry.supervisor.as_ref().is_some_and(|current| {
                    current.snapshot().package == supervisor.snapshot().package
                }) {
                    entry.failure = error
                        .as_ref()
                        .map(|error| runtime_failure(error, nonzero_incarnation(&supervisor)));
                }
            }
            self.refresh_generation_locked(&mut state)?
        };
        self.publish(published);
        Ok(self.snapshot())
    }

    pub(super) fn restart_failed_locked(
        &self,
    ) -> Result<ExtensionHostFleetSnapshot, ExtensionHostRuntimeError> {
        let supervisors = self
            .state
            .lock()
            .map_err(|_| ExtensionHostRuntimeError::Internal)?
            .entries
            .iter()
            .filter_map(|(id, entry)| {
                (entry.failure.is_some())
                    .then(|| {
                        entry
                            .supervisor
                            .clone()
                            .map(|supervisor| (id.clone(), supervisor))
                    })
                    .flatten()
            })
            .collect::<Vec<_>>();
        let outcomes = supervisors
            .into_iter()
            .map(|(id, supervisor)| {
                let error = supervisor.start().err();
                (id, supervisor, error)
            })
            .collect::<Vec<_>>();
        let published = {
            let mut state = self
                .state
                .lock()
                .map_err(|_| ExtensionHostRuntimeError::Internal)?;
            for (id, supervisor, error) in outcomes {
                let Some(entry) = state.entries.get_mut(&id) else {
                    continue;
                };
                if entry.supervisor.as_ref().is_some_and(|current| {
                    current.snapshot().package == supervisor.snapshot().package
                }) {
                    entry.failure = error
                        .as_ref()
                        .map(|error| runtime_failure(error, nonzero_incarnation(&supervisor)));
                }
            }
            self.refresh_generation_locked(&mut state)?
        };
        self.publish(published);
        Ok(self.snapshot())
    }

    pub(super) fn retire_current(
        &self,
        reason: CancelReason,
    ) -> Result<(), ExtensionHostRuntimeError> {
        let (entries, published) = {
            let mut state = self
                .state
                .lock()
                .map_err(|_| ExtensionHostRuntimeError::Internal)?;
            let entries = std::mem::take(&mut state.entries);
            let published = self.refresh_generation_locked(&mut state)?;
            (entries, published)
        };
        self.publish(published);
        let handles = self
            .sessions
            .lock()
            .map_err(|_| ExtensionHostRuntimeError::Internal)?
            .detach_all(reason);
        cancel_handles(handles, reason);
        for entry in entries.into_values() {
            if let Some(supervisor) = entry.supervisor {
                let _ = supervisor.shutdown();
            }
        }
        Ok(())
    }

    pub(super) fn refresh_generation_locked(
        &self,
        state: &mut FleetState,
    ) -> Result<Option<u64>, ExtensionHostRuntimeError> {
        let current = state
            .entries
            .values()
            .map(RuntimeEntry::projection)
            .collect::<Vec<_>>();
        if current == state.published {
            return Ok(None);
        }
        state.generation = state
            .generation
            .checked_add(1)
            .ok_or(ExtensionHostRuntimeError::Internal)?;
        state.published = current;
        Ok(Some(state.generation))
    }

    pub(super) fn snapshot(&self) -> ExtensionHostFleetSnapshot {
        let state = self
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        ExtensionHostFleetSnapshot {
            generation: state.generation,
            extensions: state.published.clone(),
        }
    }

    pub(super) fn publish(&self, generation: Option<u64>) {
        if let Some(generation) = generation {
            self.updates.publish_extension_host_changed(generation);
        }
    }
}

impl RuntimeEntry {
    fn projection(&self) -> ExtensionHostExtensionSnapshot {
        match &self.supervisor {
            Some(supervisor) => {
                extension_projection(&self.version, supervisor.snapshot(), self.failure.clone())
            }
            None => {
                let mut fallback = self.fallback.clone();
                fallback.failure = self.failure.clone();
                fallback
            }
        }
    }
}
