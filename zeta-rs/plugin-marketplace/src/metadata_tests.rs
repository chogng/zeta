use crate::RemoteMarketplaceErrorKind;
use crate::metadata::PluginTargetCatalogMetadata;
use crate::metadata::RevocationDocument;
use zeta_plugins::InstalledPluginRef;
use zeta_plugins::PluginId;
use zeta_plugins::PluginPackageDigest;
use zeta_plugins::PluginVersion;

#[test]
fn revocations_require_one_exact_digest_per_version() {
    let error = RevocationDocument::parse(
        br#"{
          "schemaVersion": 1,
          "revoked": [
            {"id":"acme/review","version":"1.0.0","digest":"sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"},
            {"id":"acme/review","version":"1.0.0","digest":"sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"}
          ]
        }"#,
    )
    .unwrap_err();

    assert_eq!(error.kind(), RemoteMarketplaceErrorKind::MetadataUntrusted);
}

#[test]
fn revocations_deduplicate_the_same_exact_package() {
    let revoked = RevocationDocument::parse(
        br#"{
          "schemaVersion": 1,
          "revoked": [
            {"id":"acme/review","version":"1.0.0","digest":"sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"},
            {"id":"acme/review","version":"1.0.0","digest":"sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"}
          ]
        }"#,
    )
    .unwrap();

    assert_eq!(revoked.len(), 1);
}

#[test]
fn catalog_metadata_binds_manifest_identity_and_bounded_package_stats() {
    let package = InstalledPluginRef {
        id: PluginId::new("acme/review").unwrap(),
        version: PluginVersion::new("1.0.0").unwrap(),
        digest: PluginPackageDigest::new(format!("sha256:{}", "a".repeat(64))).unwrap(),
    };
    let metadata: PluginTargetCatalogMetadata = serde_json::from_value(serde_json::json!({
        "schemaVersion": 1,
        "manifest": {
            "schemaVersion": 1,
            "id": "acme/review",
            "version": "1.0.0",
            "displayName": "Review",
            "compatibility": {"zeta": ">=0.1.0"},
            "contributions": {"skills": [{"id": "review", "path": "skills/review"}]},
            "permissions": []
        },
        "packageFileCount": 2,
        "packageSizeBytes": 1024
    }))
    .unwrap();

    let catalog = metadata.into_catalog(&package).unwrap();

    assert_eq!(catalog.manifest.id, package.id);
    assert_eq!(catalog.stats.file_count, 2);
    assert_eq!(catalog.stats.total_bytes, 1024);
}

#[test]
fn catalog_metadata_rejects_manifest_identity_drift() {
    let package = InstalledPluginRef {
        id: PluginId::new("acme/review").unwrap(),
        version: PluginVersion::new("1.0.0").unwrap(),
        digest: PluginPackageDigest::new(format!("sha256:{}", "a".repeat(64))).unwrap(),
    };
    let metadata: PluginTargetCatalogMetadata = serde_json::from_value(serde_json::json!({
        "schemaVersion": 1,
        "manifest": {
            "schemaVersion": 1,
            "id": "acme/other",
            "version": "1.0.0",
            "displayName": "Other",
            "compatibility": {"zeta": ">=0.1.0"},
            "contributions": {"skills": [{"id": "other", "path": "skills/other"}]},
            "permissions": []
        },
        "packageFileCount": 2,
        "packageSizeBytes": 1024
    }))
    .unwrap();

    assert!(metadata.into_catalog(&package).is_err());
}
