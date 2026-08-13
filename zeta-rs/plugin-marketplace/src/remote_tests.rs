use std::fs;

use tempfile::TempDir;
use url::Url;
use zeta_plugins::PluginMarketplaceId;

use super::RemotePluginMarketplaceConfig;
use super::recover_complete_directory;
use super::stage_datastore;

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
