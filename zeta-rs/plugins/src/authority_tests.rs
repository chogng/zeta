use std::fs;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use tempfile::tempdir;

use crate::LocalPluginPackage;
use crate::PluginActivationAuthority;
use crate::PluginAuthorityCommand;
use crate::PluginAuthorityCommandId;
use crate::PluginAuthorityCommandRequest;
use crate::PluginAuthorityDisposition;
use crate::PluginErrorKind;
use crate::PluginId;
use crate::PluginPackageStore;

fn package(root: &std::path::Path, id: &str, version: &str) -> LocalPluginPackage {
    fs::create_dir_all(root.join(".zeta-plugin")).unwrap();
    fs::create_dir_all(root.join("mcp")).unwrap();
    fs::write(
        root.join(".zeta-plugin/plugin.json"),
        format!(
            r#"{{
                "schemaVersion": 1,
                "id": "{id}",
                "version": "{version}",
                "displayName": "Test Plugin",
                "compatibility": {{"zeta": ">=0.1.0"}},
                "contributions": {{
                    "mcpServers": [{{"id": "test", "definition": "mcp/test.json"}}]
                }}
            }}"#
        ),
    )
    .unwrap();
    fs::write(root.join("mcp/test.json"), "{}").unwrap();
    LocalPluginPackage::load(root).unwrap()
}

fn request(
    authority: &PluginActivationAuthority,
    command_id: &str,
    command: PluginAuthorityCommand,
) -> PluginAuthorityCommandRequest {
    PluginAuthorityCommandRequest {
        command_id: PluginAuthorityCommandId::new(command_id).unwrap(),
        expected_revision: authority.snapshot().revision(),
        command,
    }
}

#[test]
fn authority_persists_install_enable_disable_and_exact_replay() {
    let profile = tempdir().unwrap();
    let source = tempdir().unwrap();
    let local = package(source.path(), "acme/review", "1.0.0");
    let authority = PluginActivationAuthority::open(profile.path()).unwrap();
    let install = authority
        .install_local(
            PluginAuthorityCommandId::new("install-review").unwrap(),
            0,
            &local,
        )
        .unwrap();
    let installed = install.package;
    let replayed = authority
        .install_local(
            PluginAuthorityCommandId::new("install-review").unwrap(),
            0,
            &local,
        )
        .unwrap();
    assert_eq!(install.command.revision, 1);
    assert_eq!(install.command.activation_generation, 1);
    assert_eq!(
        replayed.command.disposition,
        PluginAuthorityDisposition::Replayed
    );

    authority
        .apply(request(
            &authority,
            "enable-review",
            PluginAuthorityCommand::Enable {
                package: installed.clone(),
            },
        ))
        .unwrap();
    assert_eq!(authority.snapshot().activation().generation(), 2);
    assert_eq!(authority.snapshot().activation().packages().len(), 1);
    drop(authority);

    let reopened = PluginActivationAuthority::open(profile.path()).unwrap();
    assert_eq!(reopened.snapshot().revision(), 2);
    assert_eq!(
        reopened.snapshot().installed(),
        std::slice::from_ref(&installed)
    );
    assert_eq!(reopened.snapshot().activation().packages().len(), 1);
    reopened
        .apply(request(
            &reopened,
            "disable-review",
            PluginAuthorityCommand::Disable {
                plugin_id: installed.id,
            },
        ))
        .unwrap();
    assert!(reopened.snapshot().activation().packages().is_empty());
}

#[test]
fn failed_enable_does_not_change_the_published_generation() {
    let root = tempdir().unwrap();
    let store = PluginPackageStore::open(root.path()).unwrap();
    let authority = PluginActivationAuthority::in_memory(store).unwrap();
    let missing = crate::InstalledPluginRef {
        id: PluginId::new("acme/missing").unwrap(),
        version: crate::PluginVersion::new("1.0.0").unwrap(),
        digest: crate::PluginPackageDigest::new(format!("sha256:{}", "0".repeat(64))).unwrap(),
    };

    let error = authority
        .apply(request(
            &authority,
            "enable-missing",
            PluginAuthorityCommand::Enable { package: missing },
        ))
        .unwrap_err();
    assert_eq!(error.kind(), PluginErrorKind::SourceUnavailable);
    assert_eq!(authority.snapshot().revision(), 0);
    assert_eq!(authority.snapshot().activation().generation(), 1);
}

#[test]
fn disable_commits_then_waits_for_exact_invocation_drain() {
    let root = tempdir().unwrap();
    let source = tempdir().unwrap();
    let store = PluginPackageStore::open(root.path()).unwrap();
    let installed = store
        .install_local(&package(source.path(), "acme/review", "1.0.0"))
        .unwrap();
    let authority = PluginActivationAuthority::in_memory(store).unwrap();
    authority
        .apply(request(
            &authority,
            "install",
            PluginAuthorityCommand::Install {
                package: installed.clone(),
            },
        ))
        .unwrap();
    authority
        .apply(request(
            &authority,
            "enable",
            PluginAuthorityCommand::Enable {
                package: installed.clone(),
            },
        ))
        .unwrap();
    let active = authority.snapshot().activation().packages()[0].clone();
    let fence = authority.invocation_fence(&active).unwrap();
    let lease = fence.acquire().unwrap();
    let subscription = authority.subscribe();
    let expected_revision = authority.snapshot().revision();
    let disable_authority = authority.clone();
    let plugin_id = installed.id;
    let (completed_sender, completed_receiver) = mpsc::channel();
    let disable = thread::spawn(move || {
        let result = disable_authority.apply(PluginAuthorityCommandRequest {
            command_id: PluginAuthorityCommandId::new("disable").unwrap(),
            expected_revision,
            command: PluginAuthorityCommand::Disable { plugin_id },
        });
        completed_sender.send(()).unwrap();
        result
    });

    assert_eq!(
        subscription.recv_timeout(Duration::from_secs(1)).unwrap(),
        3
    );
    assert!(!fence.authorizes());
    assert!(matches!(
        completed_receiver.recv_timeout(Duration::from_millis(50)),
        Err(mpsc::RecvTimeoutError::Timeout)
    ));
    drop(lease);
    disable.join().unwrap().unwrap();
    completed_receiver
        .recv_timeout(Duration::from_secs(1))
        .unwrap();
}

#[test]
fn unrelated_activation_change_does_not_revoke_an_exact_package_fence() {
    let root = tempdir().unwrap();
    let first_source = tempdir().unwrap();
    let second_source = tempdir().unwrap();
    let store = PluginPackageStore::open(root.path()).unwrap();
    let first = store
        .install_local(&package(first_source.path(), "acme/first", "1.0.0"))
        .unwrap();
    let second = store
        .install_local(&package(second_source.path(), "acme/second", "1.0.0"))
        .unwrap();
    let authority = PluginActivationAuthority::in_memory(store).unwrap();
    for (id, package) in [("install-first", &first), ("install-second", &second)] {
        authority
            .apply(request(
                &authority,
                id,
                PluginAuthorityCommand::Install {
                    package: package.clone(),
                },
            ))
            .unwrap();
    }
    authority
        .apply(request(
            &authority,
            "enable-first",
            PluginAuthorityCommand::Enable {
                package: first.clone(),
            },
        ))
        .unwrap();
    let active = authority.snapshot().activation().packages()[0].clone();
    let fence = authority.invocation_fence(&active).unwrap();

    authority
        .apply(request(
            &authority,
            "enable-second",
            PluginAuthorityCommand::Enable { package: second },
        ))
        .unwrap();
    assert!(fence.authorizes());
    assert!(fence.acquire().is_some());
}
