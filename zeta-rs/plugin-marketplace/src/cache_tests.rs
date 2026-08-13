use super::RemoteMarketplaceCachePolicy;
use super::prune;
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;
use zeta_plugins::LocalPluginPackage;
use zeta_plugins::PluginPackageDigest;

#[test]
fn policy_rejects_zero_and_unbounded_limits() {
    assert!(
        RemoteMarketplaceCachePolicy::default()
            .with_max_materialized_packages(0)
            .is_err()
    );
    assert!(
        RemoteMarketplaceCachePolicy::default()
            .with_max_materialized_bytes(0)
            .is_err()
    );
    assert!(
        RemoteMarketplaceCachePolicy::default()
            .with_max_materialized_packages(4097)
            .is_err()
    );
    assert!(
        RemoteMarketplaceCachePolicy::default()
            .with_max_materialized_bytes(129 * 1024 * 1024 * 1024)
            .is_err()
    );
}

#[test]
fn evicts_unpublished_packages_before_current_signed_targets() {
    let cache = tempfile::tempdir().unwrap();
    let first = write_cached_package(cache.path(), "acme/first", "first");
    let second = write_cached_package(cache.path(), "acme/second", "second");
    let unpublished = write_cached_package(cache.path(), "acme/old", "old");
    let policy = RemoteMarketplaceCachePolicy::default()
        .with_max_materialized_packages(2)
        .unwrap();
    let published = BTreeSet::from([first.clone(), second.clone()]);

    let report = prune(cache.path(), policy, &published, &BTreeSet::new()).unwrap();

    assert_eq!(report.retained_packages, 2);
    assert_eq!(report.evicted_packages, 1);
    assert!(digest_root(cache.path(), &first).is_dir());
    assert!(digest_root(cache.path(), &second).is_dir());
    assert!(!digest_root(cache.path(), &unpublished).exists());
}

#[test]
fn protects_the_package_being_handed_to_install_and_reports_soft_excess() {
    let cache = tempfile::tempdir().unwrap();
    let protected = write_cached_package(cache.path(), "acme/protected", "protected");
    let other = write_cached_package(cache.path(), "acme/other", "other");
    let policy = RemoteMarketplaceCachePolicy::default()
        .with_max_materialized_packages(1)
        .unwrap()
        .with_max_materialized_bytes(1)
        .unwrap();

    let report = prune(
        cache.path(),
        policy,
        &BTreeSet::new(),
        &BTreeSet::from([protected.clone()]),
    )
    .unwrap();

    assert_eq!(report.retained_packages, 1);
    assert_eq!(report.excess_packages, 0);
    assert!(report.excess_bytes > 0);
    assert!(digest_root(cache.path(), &protected).is_dir());
    assert!(!digest_root(cache.path(), &other).exists());
}

#[test]
fn counts_tampered_regular_bytes_without_treating_cache_as_trust_authority() {
    let cache = tempfile::tempdir().unwrap();
    let digest = write_cached_package(cache.path(), "acme/tampered", "original");
    fs::write(
        digest_root(cache.path(), &digest).join("skills/demo/SKILL.md"),
        "tampered",
    )
    .unwrap();

    let report = prune(
        cache.path(),
        RemoteMarketplaceCachePolicy::default(),
        &BTreeSet::new(),
        &BTreeSet::new(),
    )
    .unwrap();

    assert_eq!(report.retained_packages, 1);
    assert!(digest_root(cache.path(), &digest).exists());
}

fn write_cached_package(cache: &Path, id: &str, content: &str) -> PluginPackageDigest {
    let staging = tempfile::tempdir_in(cache).unwrap();
    fs::create_dir_all(staging.path().join(".zeta-plugin")).unwrap();
    fs::create_dir_all(staging.path().join("skills/demo")).unwrap();
    fs::write(
        staging.path().join(".zeta-plugin/plugin.json"),
        format!(
            r#"{{
                "schemaVersion": 1,
                "id": "{id}",
                "version": "1.0.0",
                "displayName": "Demo",
                "compatibility": {{"zeta": ">=0.1.0"}},
                "contributions": {{"skills": [{{"id":"demo","path":"skills/demo"}}]}}
            }}"#
        ),
    )
    .unwrap();
    fs::write(staging.path().join("skills/demo/SKILL.md"), content).unwrap();
    let digest = LocalPluginPackage::load(staging.path())
        .unwrap()
        .package_digest()
        .clone();
    let packages = cache.join("packages");
    fs::create_dir_all(&packages).unwrap();
    fs::rename(staging.keep(), digest_root(cache, &digest)).unwrap();
    digest
}

fn digest_root(cache: &Path, digest: &PluginPackageDigest) -> std::path::PathBuf {
    cache.join("packages").join(
        digest
            .as_str()
            .strip_prefix("sha256:")
            .expect("validated digest prefix"),
    )
}
