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
use super::authority;
use super::authority::prepare_extension;
use super::cancel_handles;
use super::nonzero_incarnation;
use super::projection;
use super::projection::ExtensionHostExtensionSnapshot;
use super::projection::ExtensionHostFleetSnapshot;
use super::projection::extension_projection;
use super::projection::runtime_failure;

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
        let authority_snapshot = self.authority.snapshot();
        let activation = authority_snapshot.activation();
        let activation_generation = activation.generation();
        if !force
            && self
                .state
                .lock()
                .map_err(|_| ExtensionHostRuntimeError::Internal)?
                .authority_generation
                == activation_generation
        {
            return Ok(self.snapshot());
        }
        let extension_count = activation
            .packages()
            .iter()
            .map(|package| package.manifest().contributions.editor_extensions.len())
            .sum::<usize>();
        if extension_count > MAXIMUM_FLEET_EXTENSIONS {
            self.retire_current(CancelReason::AuthorityRevoked)?;
            return Err(ExtensionHostRuntimeError::QuotaExceeded);
        }
        let Some(generation) = NonZeroU64::new(activation_generation) else {
            self.retire_current(CancelReason::AuthorityRevoked)?;
            return Err(ExtensionHostRuntimeError::Internal);
        };
        self.retire_current(CancelReason::AuthorityRevoked)?;
        let mut entries = BTreeMap::new();
        for package in activation.packages() {
            for contribution in &package.manifest().contributions.editor_extensions {
                let entry = self.build_entry(&workspace, package, contribution, generation);
                entries.insert(entry.fallback.id.clone(), entry);
            }
        }
        let published = {
            let mut state = self
                .state
                .lock()
                .map_err(|_| ExtensionHostRuntimeError::Internal)?;
            state.entries = entries;
            state.authority_generation = activation_generation;
            self.refresh_generation_locked(&mut state)?
        };
        self.publish(published);
        Ok(self.snapshot())
    }

    fn build_entry(
        &self,
        workspace: &TrustedWorkspace,
        package: &zeta_plugins::InstalledPluginPackage,
        contribution: &zeta_plugins::EditorExtensionContribution,
        generation: NonZeroU64,
    ) -> RuntimeEntry {
        let id = authority::stable_extension_id(
            package.manifest().id.as_str(),
            contribution.id.as_str(),
        );
        let version = package.manifest().version.to_string();
        let fallback = ExtensionHostExtensionSnapshot {
            id,
            version: version.clone(),
            package_digest: package.package_digest().as_str().to_string(),
            runtime_api_version: contribution.runtime_api_version.as_u16(),
            activation_generation: generation.get(),
            incarnation: None,
            lifecycle: projection::ExtensionHostLifecycle::Failed,
            failure: None,
            registrations: Vec::new(),
        };
        let prepared = prepare_extension(
            &self.authority,
            workspace,
            package,
            contribution,
            generation,
        );
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
