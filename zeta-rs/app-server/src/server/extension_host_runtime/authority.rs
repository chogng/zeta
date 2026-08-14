use std::num::NonZeroU64;
use std::sync::Arc;

use zeta_editor_extension_host::ActivationAuthority;
use zeta_editor_extension_host::ActivationLease;
use zeta_editor_extension_host::ExtensionActivationSpec;
use zeta_editor_extension_host::ExtensionHostError;
use zeta_editor_extension_host::ExtensionLaunchCommand;
use zeta_workspace::TrustedWorkspace;

use super::source::EditorExtensionDeployment;

pub(super) struct PreparedExtension {
    pub(super) command: ExtensionLaunchCommand,
    pub(super) activation: ExtensionActivationSpec,
}

pub(super) fn prepare_extension(
    workspace: &TrustedWorkspace,
    deployment: &EditorExtensionDeployment,
    activation_generation: NonZeroU64,
) -> Result<PreparedExtension, ExtensionHostError> {
    workspace
        .ensure_active()
        .map_err(|_| ExtensionHostError::AuthorityDenied)?;
    if !deployment.authority.authorizes() {
        return Err(ExtensionHostError::AuthorityDenied);
    }
    let authority: Arc<dyn ActivationAuthority> = Arc::new(WorkspaceActivationAuthority {
        source: Arc::clone(&deployment.authority),
        workspace: workspace.clone(),
    });
    Ok(PreparedExtension {
        command: deployment.command.clone(),
        activation: ExtensionActivationSpec::new(
            deployment.params.clone(),
            activation_generation,
            authority,
        ),
    })
}

struct WorkspaceActivationAuthority {
    source: Arc<dyn ActivationAuthority>,
    workspace: TrustedWorkspace,
}

impl ActivationAuthority for WorkspaceActivationAuthority {
    fn authorizes(&self) -> bool {
        self.workspace.ensure_active().is_ok() && self.source.authorizes()
    }

    fn acquire(&self) -> Option<Box<dyn ActivationLease>> {
        self.workspace.ensure_active().ok()?;
        let source = self.source.acquire()?;
        if self.workspace.ensure_active().is_err() {
            drop(source);
            return None;
        }
        Some(Box::new(WorkspaceActivationLease {
            _source: source,
            _workspace: self.workspace.clone(),
        }))
    }
}

struct WorkspaceActivationLease {
    _source: Box<dyn ActivationLease>,
    _workspace: TrustedWorkspace,
}

impl ActivationLease for WorkspaceActivationLease {}
