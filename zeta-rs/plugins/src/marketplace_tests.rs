use std::fs;

use tempfile::tempdir;

use super::PluginMarketplace;
use super::PluginMarketplaceMode;
use super::PluginMarketplaceService;
use super::PluginProfileRequest;
use super::PluginProfileRequestEnablement;
use crate::LocalPluginPackage;
use crate::PluginActivationAuthority;
use crate::PluginAuthorityCommand;
use crate::PluginAuthorityCommandId;
use crate::PluginAuthorityCommandRequest;
use crate::PluginErrorKind;

fn write_package(root: &std::path::Path, version: &str, body: &str) -> LocalPluginPackage {
    fs::create_dir_all(root.join(".zeta-plugin")).unwrap();
    fs::create_dir_all(root.join("skills/review")).unwrap();
    fs::write(root.join("skills/review/SKILL.md"), body).unwrap();
    fs::write(
        root.join(".zeta-plugin/plugin.json"),
        format!(
            r#"{{
                "schemaVersion": 1,
                "id": "acme/review",
                "version": "{version}",
                "displayName": "Acme Review",
                "compatibility": {{"zeta": ">=0.1.0"}},
                "contributions": {{"skills": [{{"id": "review", "path": "skills/review"}}]}}
            }}"#
        ),
    )
    .unwrap();
    LocalPluginPackage::load(root).unwrap()
}

fn write_catalog(root: &std::path::Path, packages: &[(&str, &LocalPluginPackage)]) {
    fs::create_dir_all(root.join(".zeta-marketplace")).unwrap();
    let entries = packages
        .iter()
        .map(|(path, package)| {
            format!(
                r#"{{"id":"{}","version":"{}","digest":"{}","path":"{}"}}"#,
                package.manifest().id,
                package.manifest().version,
                package.package_digest(),
                path
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    fs::write(
        root.join(".zeta-marketplace/marketplace.json"),
        format!(r#"{{"schemaVersion":1,"id":"acme","plugins":[{entries}]}}"#),
    )
    .unwrap();
}

#[test]
fn marketplace_install_stages_digest_pinned_content_without_granting_it() {
    let root = tempdir().unwrap();
    let first = write_package(&root.path().join("packages/review-1"), "1.0.0", "# One");
    write_catalog(root.path(), &[("packages/review-1", &first)]);
    let marketplace = PluginMarketplace::open(root.path(), PluginMarketplaceMode::Managed).unwrap();
    let package = marketplace.list()[0].package_ref();
    let authority = PluginActivationAuthority::open(root.path().join("profile")).unwrap();
    let service = PluginMarketplaceService::new(authority.clone(), [marketplace]).unwrap();

    service
        .install(
            PluginAuthorityCommandId::new("install").unwrap(),
            0,
            &super::PluginMarketplaceId::new("acme").unwrap(),
            &package,
        )
        .unwrap();

    assert_eq!(authority.snapshot().installed(), &[package]);
    assert!(authority.snapshot().granted().is_empty());
    assert!(authority.snapshot().activation().packages().is_empty());
}

#[test]
fn update_stages_new_bytes_and_rollback_requires_old_digest_grant() {
    let root = tempdir().unwrap();
    let first = write_package(&root.path().join("packages/review-1"), "1.0.0", "# One");
    let second = write_package(&root.path().join("packages/review-2"), "2.0.0", "# Two");
    write_catalog(
        root.path(),
        &[
            ("packages/review-1", &first),
            ("packages/review-2", &second),
        ],
    );
    let marketplace = PluginMarketplace::open(root.path(), PluginMarketplaceMode::Managed).unwrap();
    let old = marketplace.list()[0].package_ref();
    let new = marketplace.list()[1].package_ref();
    let authority = PluginActivationAuthority::open(root.path().join("profile")).unwrap();
    let service = PluginMarketplaceService::new(authority.clone(), [marketplace]).unwrap();
    let marketplace_id = super::PluginMarketplaceId::new("acme").unwrap();
    service
        .install(
            PluginAuthorityCommandId::new("install-old").unwrap(),
            0,
            &marketplace_id,
            &old,
        )
        .unwrap();
    for (id, command) in [
        (
            "grant-old",
            PluginAuthorityCommand::Grant {
                package: old.clone(),
            },
        ),
        (
            "enable-old",
            PluginAuthorityCommand::Enable {
                package: old.clone(),
            },
        ),
    ] {
        authority
            .apply(PluginAuthorityCommandRequest {
                command_id: PluginAuthorityCommandId::new(id).unwrap(),
                expected_revision: authority.snapshot().revision(),
                command,
            })
            .unwrap();
    }
    service
        .stage_update(
            PluginAuthorityCommandId::new("stage-new").unwrap(),
            authority.snapshot().revision(),
            &marketplace_id,
            &new,
        )
        .unwrap();
    assert_eq!(authority.snapshot().enabled(), std::slice::from_ref(&old));

    let error = service
        .rollback(
            PluginAuthorityCommandId::new("bad-rollback").unwrap(),
            authority.snapshot().revision(),
            old.clone(),
        )
        .unwrap_err();
    assert_eq!(error.kind(), PluginErrorKind::SourceUnavailable);

    for (id, command) in [
        (
            "grant-new",
            PluginAuthorityCommand::Grant {
                package: new.clone(),
            },
        ),
        (
            "enable-new",
            PluginAuthorityCommand::Enable {
                package: new.clone(),
            },
        ),
    ] {
        authority
            .apply(PluginAuthorityCommandRequest {
                command_id: PluginAuthorityCommandId::new(id).unwrap(),
                expected_revision: authority.snapshot().revision(),
                command,
            })
            .unwrap();
    }
    service
        .rollback(
            PluginAuthorityCommandId::new("rollback").unwrap(),
            authority.snapshot().revision(),
            old.clone(),
        )
        .unwrap();
    assert_eq!(authority.snapshot().enabled(), &[old]);
}

#[test]
fn catalog_rejects_a_digest_that_does_not_match_package_content() {
    let root = tempdir().unwrap();
    let package = write_package(&root.path().join("packages/review"), "1.0.0", "# One");
    write_catalog(root.path(), &[("packages/review", &package)]);
    let catalog = root.path().join(".zeta-marketplace/marketplace.json");
    let contents = fs::read_to_string(&catalog).unwrap();
    fs::write(
        &catalog,
        contents.replace(
            package.package_digest().as_str(),
            &format!("sha256:{}", "0".repeat(64)),
        ),
    )
    .unwrap();

    let error = PluginMarketplace::open(root.path(), PluginMarketplaceMode::Managed).unwrap_err();

    assert_eq!(error.kind(), PluginErrorKind::SourceUnavailable);
}

#[cfg(unix)]
#[test]
fn marketplace_rejects_linked_catalog_or_package_path_components() {
    use std::os::unix::fs::symlink;

    let root = tempdir().unwrap();
    let external = tempdir().unwrap();
    let package = write_package(&external.path().join("review"), "1.0.0", "# One");
    fs::create_dir_all(root.path().join(".zeta-marketplace")).unwrap();
    symlink(
        external.path().join("review"),
        root.path().join("linked-package"),
    )
    .unwrap();
    write_catalog(root.path(), &[("linked-package", &package)]);

    let error = PluginMarketplace::open(root.path(), PluginMarketplaceMode::Managed).unwrap_err();

    assert_eq!(error.kind(), PluginErrorKind::SourceUnavailable);
}

#[test]
fn profile_reconcile_installs_and_enables_without_implicitly_granting() {
    let root = tempdir().unwrap();
    let package = write_package(&root.path().join("packages/review"), "1.0.0", "# One");
    write_catalog(root.path(), &[("packages/review", &package)]);
    let marketplace = PluginMarketplace::open(root.path(), PluginMarketplaceMode::Managed).unwrap();
    let authority = PluginActivationAuthority::open(root.path().join("profile")).unwrap();
    let service = PluginMarketplaceService::new(authority.clone(), [marketplace]).unwrap();

    let resolutions = service
        .reconcile_profile([PluginProfileRequest {
            id: package.manifest().id.clone(),
            version: package.manifest().version.clone(),
            enablement: PluginProfileRequestEnablement::Enabled,
        }])
        .unwrap();

    assert_eq!(resolutions.len(), 1);
    assert!(resolutions[0].installed);
    assert!(resolutions[0].enabled);
    assert!(!resolutions[0].granted);
    assert!(!resolutions[0].effective);
    assert!(authority.snapshot().granted().is_empty());
}
