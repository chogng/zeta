use semver::Version;

use crate::LanguageMarketplaceCompatibility;
use crate::LanguageMarketplaceId;
use crate::LanguageMarketplaceRuntime;
use crate::model::CatalogContext;
use crate::model::PackageCatalogMetadata;
use crate::model::PackageTargetMetadata;
use crate::model::catalog_entries;

#[test]
fn schema_one_css_catalog_maps_to_one_legacy_node_adapter_route() {
    let marketplace = LanguageMarketplaceId::new("official").unwrap();
    let (package, catalog) = css_metadata(None);
    let entries = catalog_entries(CatalogContext {
        marketplace_id: &marketplace,
        package,
        catalog,
        target_name: "packages/marketplace/css/1.0.0.zip",
        target_length: 1024,
        consumer_id: "zeta",
        consumer_version: &Version::new(0, 1, 0),
    })
    .unwrap();

    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].server_id(), "css-language-server");
    assert_eq!(
        entries[0].runtime(),
        LanguageMarketplaceRuntime::LegacyUnspecified
    );
    assert_eq!(entries[0].languages(), &["css", "less", "scss"]);
    assert!(entries[0].compatibility().is_compatible());
}

#[test]
fn signed_consumer_requirement_marks_entry_incompatible() {
    let marketplace = LanguageMarketplaceId::new("official").unwrap();
    let (package, catalog) = css_metadata(Some(">=2.0.0"));
    let entries = catalog_entries(CatalogContext {
        marketplace_id: &marketplace,
        package,
        catalog,
        target_name: "packages/marketplace/css/1.0.0.zip",
        target_length: 1024,
        consumer_id: "zeta",
        consumer_version: &Version::new(0, 1, 0),
    })
    .unwrap();

    assert!(matches!(
        entries[0].compatibility(),
        LanguageMarketplaceCompatibility::Incompatible(reason)
            if reason.contains(">=2.0.0")
    ));
}

fn css_metadata(compatibility: Option<&str>) -> (PackageTargetMetadata, PackageCatalogMetadata) {
    let consumer = compatibility.map_or_else(String::new, |requirement| {
        format!(
            r#", "consumers": {{"zeta": {{"compatibility": "{requirement}", "metadataPath": ".marketplace/consumers/zeta.json"}}}}"#
        )
    });
    let identity = serde_json::json!({
        "schemaVersion": 1,
        "id": "marketplace/css",
        "version": "1.0.0",
        "packageDigest": format!("sha256:{}", "a".repeat(64)),
    });
    let catalog = format!(
        r#"{{
          "schemaVersion": 1,
          "manifest": {{
            "schemaVersion": 1,
            "packageType": "language",
            "source": "official",
            "id": "marketplace/css",
            "version": "1.0.0",
            "displayName": "CSS Language Support",
            "description": "CSS, Less and SCSS support",
            "license": "MIT",
            "languages": [
              {{"id":"css","displayName":"CSS","aliases":["css"],"fileExtensions":[".css"],"lsp":true}},
              {{"id":"less","displayName":"Less","aliases":["less"],"fileExtensions":[".less"],"lsp":true}},
              {{"id":"scss","displayName":"SCSS","aliases":["scss"],"fileExtensions":[".scss"],"lsp":true}}
            ],
            "capabilities": [
              {{"kind":"asset","id":"language-assets","path":"language"}},
              {{"kind":"executable","id":"css-language-server","path":"server/css-language-server"}}
            ]
            {consumer}
          }},
          "consumerMetadata": {{}},
          "packageFileCount": 4,
          "packageSizeBytes": 128
        }}"#
    );
    (
        serde_json::from_value(identity).unwrap(),
        serde_json::from_str(&catalog).unwrap(),
    )
}
