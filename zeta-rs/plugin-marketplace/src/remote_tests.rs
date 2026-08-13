use std::fs;

use tempfile::TempDir;
use url::Url;
use zeta_plugins::PluginMarketplaceId;
use zeta_plugins::PluginMarketplaceTrust;
use zeta_plugins::PluginPackageDigest;

use super::RemotePluginMarketplaceConfig;
use super::cache_coordinator;
use super::recover_complete_directory;
use super::stage_datastore;

#[test]
fn cache_coordinator_is_shared_and_tracks_active_materialization_leases() {
    let root = TempDir::new().unwrap();
    let first = cache_coordinator(root.path());
    let second = cache_coordinator(root.path());
    let digest = PluginPackageDigest::new(format!("sha256:{}", "a".repeat(64))).unwrap();

    assert!(std::sync::Arc::ptr_eq(&first, &second));
    let lease = first.lease(&digest).unwrap();
    assert_eq!(second.protected_digests().unwrap(), [digest.clone()].into());

    drop(lease);
    assert!(second.protected_digests().unwrap().is_empty());
}

#[test]
fn cache_recovery_restores_the_previous_complete_repository() {
    let root = TempDir::new().unwrap();
    let backup = root.path().join("repository.previous");
    fs::create_dir(&backup).unwrap();
    fs::write(backup.join("complete"), b"old").unwrap();

    recover_complete_directory(root.path(), "repository").unwrap();

    assert_eq!(
        fs::read(root.path().join("repository/complete")).unwrap(),
        b"old"
    );
    assert!(!backup.exists());
}

#[test]
fn cache_recovery_keeps_the_promoted_repository() {
    let root = TempDir::new().unwrap();
    let repository = root.path().join("repository");
    let backup = root.path().join("repository.previous");
    fs::create_dir(&repository).unwrap();
    fs::create_dir(&backup).unwrap();
    fs::write(repository.join("complete"), b"new").unwrap();
    fs::write(backup.join("complete"), b"old").unwrap();

    recover_complete_directory(root.path(), "repository").unwrap();

    assert_eq!(fs::read(repository.join("complete")).unwrap(), b"new");
    assert!(!backup.exists());
}

#[test]
fn staged_rollback_state_does_not_mutate_the_committed_datastore() {
    let root = TempDir::new().unwrap();
    let current = root.path().join("tuf");
    fs::create_dir(&current).unwrap();
    fs::write(current.join("timestamp.json"), b"old").unwrap();

    let staging = stage_datastore(root.path(), &current).unwrap();
    fs::write(staging.path().join("timestamp.json"), b"new").unwrap();
    drop(staging);

    assert_eq!(fs::read(current.join("timestamp.json")).unwrap(), b"old");
}

#[test]
fn distribution_urls_require_unambiguous_https_directory_bases() {
    let root = TempDir::new().unwrap();
    let configuration = |metadata: &str, targets: &str| {
        RemotePluginMarketplaceConfig::new(
            PluginMarketplaceId::new("zeta-test").unwrap(),
            Url::parse(metadata).unwrap(),
            Url::parse(targets).unwrap(),
            b"trusted root".to_vec(),
            root.path(),
        )
    };

    assert!(
        configuration(
            "https://marketplace.example/metadata",
            "https://marketplace.example/targets/"
        )
        .is_err()
    );
    assert!(
        configuration(
            "https://user@marketplace.example/metadata/",
            "https://marketplace.example/targets/"
        )
        .is_err()
    );
    assert!(
        configuration(
            "https://marketplace.example/metadata/",
            "https://marketplace.example/targets/?channel=test"
        )
        .is_err()
    );
}

#[test]
fn external_source_trust_requires_explicit_valid_publisher_namespaces() {
    let root = TempDir::new().unwrap();
    let config = RemotePluginMarketplaceConfig::new(
        PluginMarketplaceId::new("community").unwrap(),
        Url::parse("https://community.example/metadata/").unwrap(),
        Url::parse("https://community.example/targets/").unwrap(),
        b"trusted root".to_vec(),
        root.path(),
    )
    .unwrap();

    assert_eq!(config.trust(), PluginMarketplaceTrust::ProductManaged);
    assert_eq!(
        config
            .clone()
            .with_verified_external_publishers(["community".to_owned()])
            .unwrap()
            .trust(),
        PluginMarketplaceTrust::VerifiedExternal
    );
    assert!(
        config
            .clone()
            .with_verified_external_publishers(Vec::new())
            .is_err()
    );
    assert!(
        config
            .clone()
            .with_verified_external_publishers(["Community".to_owned()])
            .is_err()
    );
    assert!(
        config
            .with_verified_external_publishers(["community".to_owned(), "community".to_owned()])
            .is_err()
    );
}
