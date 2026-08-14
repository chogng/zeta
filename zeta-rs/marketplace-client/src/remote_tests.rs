use tempfile::TempDir;
use url::Url;

use super::RemoteMarketplaceConfig;

#[test]
fn production_registry_requires_https_and_a_trusted_root() {
    let cache = TempDir::new().unwrap();
    let metadata = Url::parse("http://marketplace.example/metadata/").unwrap();
    let targets = Url::parse("https://marketplace.example/targets/").unwrap();

    assert!(
        RemoteMarketplaceConfig::new(metadata, targets.clone(), b"root".to_vec(), cache.path())
            .is_err()
    );
    assert!(
        RemoteMarketplaceConfig::new(
            Url::parse("https://marketplace.example/metadata/").unwrap(),
            targets,
            Vec::new(),
            cache.path(),
        )
        .is_err()
    );
}

#[test]
fn production_registry_exposes_only_pinned_remote_endpoints() {
    let cache = TempDir::new().unwrap();
    let metadata = Url::parse("https://marketplace.example/metadata/").unwrap();
    let targets = Url::parse("https://marketplace.example/targets/").unwrap();
    let config = RemoteMarketplaceConfig::new(
        metadata.clone(),
        targets.clone(),
        b"root".to_vec(),
        cache.path(),
    )
    .unwrap()
    .with_allowed_publishers(["example".to_owned()])
    .unwrap();

    assert_eq!(config.metadata_base_url(), &metadata);
    assert_eq!(config.targets_base_url(), &targets);
    assert!(
        config
            .clone()
            .with_allowed_publishers(["example".to_owned(), "example".to_owned()])
            .is_err()
    );
}
