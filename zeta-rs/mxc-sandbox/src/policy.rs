use crate::unavailable;
use mxc_sdk::NetworkAction;
use mxc_sdk::NetworkEgressSection;
use mxc_sdk::NetworkIngressSection;
use mxc_sdk::policy::FilesystemSection;
use mxc_sdk::policy::HostFilesystemAccess;
use mxc_sdk::policy::NetworkSection;
use std::path::Path;
use zeta_sandboxing::FileSystemAccess;
use zeta_sandboxing::HostAclChanges;
use zeta_sandboxing::NetworkAccess;
use zeta_sandboxing::SandboxCommand;
use zeta_sandboxing::SandboxDirAccess;
use zeta_sandboxing::SandboxError;
use zeta_sandboxing::SandboxPolicy;
use zeta_sandboxing::SandboxScope;

pub(super) fn request(
    command: &SandboxCommand,
    policy: SandboxPolicy,
    scope: &SandboxScope,
) -> Result<mxc_sdk::SandboxRequest, SandboxError> {
    validate_paths(command.working_directory(), scope)?;
    let filesystem = filesystem(policy.file_system(), scope)?;
    let action = if policy.network() == NetworkAccess::Allowed {
        NetworkAction::Allow
    } else {
        NetworkAction::Deny
    };
    let mut egress = NetworkEgressSection::default();
    egress.default = Some(action);
    let mut ingress = NetworkIngressSection::default();
    ingress.default = Some(action);
    ingress.host_loopback = Some(action);
    let mut network = NetworkSection::default();
    network.egress = Some(egress);
    network.ingress = Some(ingress);
    match (policy.network(), command.network_proxy()) {
        (NetworkAccess::Managed, Some(proxy)) if proxy.ports()[0] == proxy.ports()[1] => {
            network = NetworkSection::managed_proxy(
                proxy.ports()[0]
                    .try_into()
                    .map_err(|_| unavailable("proxy port must be nonzero"))?,
            );
        }
        (NetworkAccess::Allowed | NetworkAccess::Denied, None) => {}
        _ => {
            return Err(unavailable(
                "managed networking requires one execution-owned HTTP/SOCKS endpoint",
            ));
        }
    }
    let policy_input = mxc_sdk::SandboxPolicy {
        version: "0.8.0-alpha".into(),
        filesystem: Some(filesystem),
        network: Some(network),
        ui: None,
        timeout_ms: None,
    };
    let mut request = mxc_sdk::build_request(&policy_input, None)
        .map_err(|error| unavailable(error.to_string()))?;
    request
        .set_host_filesystem(if policy.file_system() == FileSystemAccess::FullAccess {
            HostFilesystemAccess::ReadWrite
        } else {
            HostFilesystemAccess::ReadOnly
        })
        .map_err(|error| unavailable(error.to_string()))?;
    match policy.host_acl_changes() {
        HostAclChanges::Denied => {
            request.forbid_host_acl_changes();
        }
        HostAclChanges::Scoped | HostAclChanges::ScopedWithTraversal => {
            let roots = scope
                .grants()
                .iter()
                .map(|grant| grant.dir().canonical_path().to_owned())
                .chain(
                    scope
                        .hidden_dirs()
                        .iter()
                        .map(|dir| dir.canonical_path().to_owned()),
                )
                .collect::<Vec<_>>();
            request
                .permit_host_acl_changes(&roots)
                .map_err(|error| unavailable(error.to_string()))?;
        }
    }
    #[cfg(target_os = "macos")]
    if policy.network() != NetworkAccess::Allowed {
        request.deny_seatbelt_unix_sockets();
    }
    let argv = std::iter::once(command.program())
        .chain(command.arguments().iter().map(|arg| arg.as_os_str()))
        .map(|arg| {
            arg.to_str()
                .map(str::to_owned)
                .ok_or_else(|| unavailable("MXC requires Unicode command arguments"))
        })
        .collect::<Result<Vec<_>, _>>()?;
    request
        .set_command(&argv)
        .map_err(|error| unavailable(error.to_string()))?;
    request.set_working_directory(text(command.working_directory())?);
    Ok(request)
}

fn filesystem(
    access: FileSystemAccess,
    scope: &SandboxScope,
) -> Result<FilesystemSection, SandboxError> {
    let mut filesystem = FilesystemSection::default();
    for grant in scope.grants() {
        let root = text(grant.dir().canonical_path())?;
        if access != FileSystemAccess::ReadOnly && grant.access() == SandboxDirAccess::ReadWrite {
            filesystem.readwrite_paths.push(root);
            for name in zeta_sandboxing::PROTECTED_DIR_METADATA_NAMES {
                let path = grant.dir().canonical_path().join(name);
                // The contract protects existing metadata. Do not ask an ACL
                // backend to open absent paths, or hide inspection failures.
                match std::fs::symlink_metadata(&path) {
                    Ok(_) => filesystem.readonly_paths.push(text(&path)?),
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                    Err(error) => {
                        return Err(unavailable(format!(
                            "cannot inspect protected path '{}': {error}",
                            path.display()
                        )));
                    }
                }
            }
        } else {
            filesystem.readonly_paths.push(root);
        }
    }
    filesystem.denied_paths = scope
        .hidden_dirs()
        .iter()
        .map(|dir| text(dir.canonical_path()))
        .collect::<Result<_, _>>()?;
    filesystem.clear_policy_on_exit = Some(true);
    Ok(filesystem)
}

pub(super) fn validate_paths(cwd: &Path, scope: &SandboxScope) -> Result<(), SandboxError> {
    for path in std::iter::once(cwd)
        .chain(
            scope
                .grants()
                .iter()
                .map(|grant| grant.dir().canonical_path()),
        )
        .chain(scope.hidden_dirs().iter().map(|dir| dir.canonical_path()))
    {
        if dunce::canonicalize(path).map_err(|error| unavailable(error.to_string()))? != path {
            return Err(unavailable("sandbox directory changed since preparation"));
        }
    }
    Ok(())
}
fn text(path: &Path) -> Result<String, SandboxError> {
    path.to_str()
        .filter(|value| !value.contains('\0'))
        .map(str::to_owned)
        .ok_or_else(|| unavailable("MXC requires Unicode filesystem paths without NUL"))
}

#[cfg(test)]
#[path = "policy_tests.rs"]
mod tests;
