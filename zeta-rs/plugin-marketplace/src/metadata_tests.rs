use crate::RemoteMarketplaceErrorKind;
use crate::metadata::MarketplaceTargetCatalogMetadata;
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
fn generic_catalog_extracts_only_the_zeta_consumer_adapter() {
    let package = package("acme/review");
    let metadata: MarketplaceTargetCatalogMetadata = serde_json::from_value(serde_json::json!({
        "schemaVersion": 1,
        "manifest": {
            "schemaVersion": 1,
            "id": "acme/review",
            "version": "1.0.0",
            "displayName": "Review",
            "description": "Portable review workflow.",
            "license": "MIT",
            "capabilities": [{"kind": "skill", "id": "review", "path": "skills/review"}],
            "consumers": {"zeta": {"metadataPath": ".zeta-plugin/plugin.json"}}
        },
        "consumerMetadata": {
            "zeta": {
                "schemaVersion": 1,
                "id": "acme/review",
                "version": "1.0.0",
                "displayName": "Review",
                "compatibility": {"zeta": ">=0.1.0"},
                "contributions": {"skills": [{"id": "review", "path": "skills/review"}]},
                "permissions": []
            }
        },
        "packageFileCount": 2,
        "packageSizeBytes": 1024
    }))
    .unwrap();

    let catalog = metadata.into_zeta_catalog(&package).unwrap().unwrap();

    assert_eq!(catalog.manifest.id, package.id);
    assert_eq!(catalog.stats.file_count, 2);
    assert_eq!(catalog.stats.total_bytes, 1024);
}

#[test]
fn generic_catalog_without_zeta_adapter_is_ignored() {
    let package = package("acme/review");
    let metadata: MarketplaceTargetCatalogMetadata = serde_json::from_value(serde_json::json!({
        "schemaVersion": 1,
        "manifest": {
            "schemaVersion": 1,
            "id": "acme/review",
            "version": "1.0.0",
            "displayName": "Review",
            "description": "Portable review workflow.",
            "license": "MIT",
            "capabilities": [{"kind": "skill", "id": "review", "path": "skills/review"}]
        },
        "consumerMetadata": {},
        "packageFileCount": 2,
        "packageSizeBytes": 1024
    }))
    .unwrap();

    assert!(metadata.into_zeta_catalog(&package).unwrap().is_none());
}

fn package(id: &str) -> InstalledPluginRef {
    InstalledPluginRef {
        id: PluginId::new(id).unwrap(),
        version: PluginVersion::new("1.0.0").unwrap(),
        digest: PluginPackageDigest::new(format!("sha256:{}", "a".repeat(64))).unwrap(),
    }
}
