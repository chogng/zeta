use std::collections::BTreeMap;
use std::num::NonZeroU64;
use std::sync::Arc;

use zeta_editor_extension_host::ActivateParams;
use zeta_editor_extension_host::ActivationAuthority;
use zeta_editor_extension_host::ActivationLease;
use zeta_editor_extension_host::ExtensionActivationSpec;
use zeta_editor_extension_host::ExtensionCapability;
use zeta_editor_extension_host::ExtensionHostError;
use zeta_editor_extension_host::ExtensionLaunchCommand;
use zeta_editor_extension_host::PackageBinding;
use zeta_plugins::EditorExtensionActivationEvent;
use zeta_plugins::EditorExtensionCapability;
use zeta_plugins::EditorExtensionContribution;
use zeta_plugins::InstalledPluginPackage;
use zeta_plugins::PluginActivationAuthority;
use zeta_plugins::PluginInvocationFence;
use zeta_plugins::PluginInvocationLease;
use zeta_workspace::TrustedWorkspace;

pub(super) struct PreparedExtension {
    pub(super) command: ExtensionLaunchCommand,
    pub(super) activation: ExtensionActivationSpec,
}

pub(super) fn prepare_extension(
    authority: &PluginActivationAuthority,
    workspace: &TrustedWorkspace,
    package: &InstalledPluginPackage,
    contribution: &EditorExtensionContribution,
    activation_generation: NonZeroU64,
) -> Result<PreparedExtension, ExtensionHostError> {
    workspace
        .ensure_active()
        .map_err(|_| ExtensionHostError::AuthorityDenied)?;
    let fence = authority
        .invocation_fence(package)
        .ok_or(ExtensionHostError::AuthorityDenied)?;
    let extension_id =
        stable_extension_id(package.manifest().id.as_str(), contribution.id.as_str());
    let executable = package
        .resolve_file(&contribution.entrypoint)
        .map_err(|_| ExtensionHostError::SpawnFailed)?;
    let command = ExtensionLaunchCommand::new(
        executable,
        std::iter::empty::<String>(),
        package.package_root(),
        BTreeMap::new(),
    )?;
    let params = ActivateParams {
        extension_id: extension_id.clone(),
        package: PackageBinding {
            package_id: format!("{}@{}", package.manifest().id, package.manifest().version),
            package_digest: package.package_digest().as_str().to_string(),
            entrypoint: contribution.entrypoint.as_str().to_string(),
        },
        runtime_api_version: contribution.runtime_api_version.as_u16(),
        activation_events: contribution
            .activation_events
            .iter()
            .map(activation_event)
            .collect(),
        capabilities: contribution
            .capabilities
            .iter()
            .copied()
            .map(extension_capability)
            .collect(),
    };
    let authority: Arc<dyn ActivationAuthority> = Arc::new(PluginWorkspaceAuthority {
        plugin: fence,
        workspace: workspace.clone(),
    });
    Ok(PreparedExtension {
        command,
        activation: ExtensionActivationSpec::new(params, activation_generation, authority),
    })
}

pub(super) fn stable_extension_id(plugin_id: &str, contribution_id: &str) -> String {
    format!("{plugin_id}:{contribution_id}")
}

fn activation_event(event: &EditorExtensionActivationEvent) -> String {
    match event {
        EditorExtensionActivationEvent::Startup => "startup".into(),
        EditorExtensionActivationEvent::OnCommand { id } => format!("onCommand:{id}"),
        EditorExtensionActivationEvent::OnLanguage { id } => format!("onLanguage:{id}"),
        EditorExtensionActivationEvent::OnDemand { capability } => {
            format!("onDemand:{}", capability_name(*capability))
        }
        EditorExtensionActivationEvent::OnDebugType { debug_type } => {
            format!("onDebugType:{debug_type}")
        }
        EditorExtensionActivationEvent::OnTaskType { task_type } => {
            format!("onTaskType:{task_type}")
        }
        EditorExtensionActivationEvent::OnTestProfile { profile_id } => {
            format!("onTestProfile:{profile_id}")
        }
    }
}

fn extension_capability(capability: EditorExtensionCapability) -> ExtensionCapability {
    match capability {
        EditorExtensionCapability::Command => ExtensionCapability::Command,
        EditorExtensionCapability::LanguageProvider => ExtensionCapability::LanguageProvider,
        EditorExtensionCapability::DebugAdapter => ExtensionCapability::DebugAdapter,
        EditorExtensionCapability::TaskProvider => ExtensionCapability::TaskProvider,
        EditorExtensionCapability::TestProfileProvider => ExtensionCapability::TestProfileProvider,
    }
}

fn capability_name(capability: EditorExtensionCapability) -> &'static str {
    match capability {
        EditorExtensionCapability::Command => "command",
        EditorExtensionCapability::LanguageProvider => "languageProvider",
        EditorExtensionCapability::DebugAdapter => "debugAdapter",
        EditorExtensionCapability::TaskProvider => "taskProvider",
        EditorExtensionCapability::TestProfileProvider => "testProfileProvider",
    }
}

struct PluginWorkspaceAuthority {
    plugin: PluginInvocationFence,
    workspace: TrustedWorkspace,
}

impl ActivationAuthority for PluginWorkspaceAuthority {
    fn authorizes(&self) -> bool {
        self.workspace.ensure_active().is_ok() && self.plugin.authorizes()
    }

    fn acquire(&self) -> Option<Box<dyn ActivationLease>> {
        self.workspace.ensure_active().ok()?;
        let plugin = self.plugin.acquire()?;
        if self.workspace.ensure_active().is_err() {
            drop(plugin);
            return None;
        }
        Some(Box::new(PluginWorkspaceLease {
            _plugin: plugin,
            _workspace: self.workspace.clone(),
        }))
    }
}

struct PluginWorkspaceLease {
    _plugin: PluginInvocationLease,
    _workspace: TrustedWorkspace,
}

impl ActivationLease for PluginWorkspaceLease {}
