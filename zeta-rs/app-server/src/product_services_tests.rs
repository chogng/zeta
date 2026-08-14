use std::fs;

use super::*;
use tempfile::TempDir;

#[test]
fn product_services_loads_public_oauth_and_pins_marketplace_root() {
    let root = TempDir::new().unwrap();
    fs::write(root.path().join("root.json"), br#"{"signed":{}}"#).unwrap();
    fs::write(
        root.path().join("product-services.json"),
        r#"{
          "schemaVersion": 1,
          "marketplaceManager": {
            "metadataBaseUrl": "https://marketplace.zeta.example/metadata/",
            "targetsBaseUrl": "https://marketplace.zeta.example/targets/",
            "trustedRoot": "root.json"
          },
          "connectorOauth": [{
            "type": "githubDevice",
            "connectorId": "openai/github:connector:account",
            "clientId": "public-client-id",
            "scopes": ["read:user", "repo"]
          }]
        }"#,
    )
    .unwrap();

    let config = LocalProductServicesConfig::load(
        root.path().join("product-services.json"),
        root.path().join("profile"),
    )
    .unwrap();

    assert!(config.marketplace_registry().is_some());
    assert!(matches!(
        &config.connector_oauth[0],
        ProductConnectorOAuthConfig::GitHubDevice { connector_id, config }
            if connector_id.as_str() == "openai/github:connector:account"
                && config.client_id == "public-client-id"
    ));
}

#[test]
fn production_product_services_delegates_to_the_marketplace_manager() {
    let product_services = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../resources/product-services/product-services.json");
    let profile = TempDir::new().unwrap();

    let config = LocalProductServicesConfig::load(product_services, profile.path()).unwrap();

    let registry = config.marketplace_registry().unwrap();
    assert_eq!(
        registry.metadata_base_url().as_str(),
        "https://chogng.github.io/marketplace/metadata/"
    );
    assert_eq!(
        registry.targets_base_url().as_str(),
        "https://chogng.github.io/marketplace/targets/"
    );
}

#[cfg(unix)]
#[test]
fn product_services_rejects_symlinked_trust_inputs() {
    use std::os::unix::fs::symlink;

    let root = TempDir::new().unwrap();
    fs::write(root.path().join("actual.json"), br#"{"schemaVersion":1}"#).unwrap();
    symlink(
        root.path().join("actual.json"),
        root.path().join("product-services.json"),
    )
    .unwrap();

    assert!(
        LocalProductServicesConfig::load(
            root.path().join("product-services.json"),
            root.path().join("profile"),
        )
        .is_err()
    );
}

#[test]
fn product_services_rejects_trust_roots_outside_its_directory() {
    let parent = TempDir::new().unwrap();
    let product = parent.path().join("product");
    fs::create_dir(&product).unwrap();
    fs::write(parent.path().join("root.json"), br#"{"signed":{}}"#).unwrap();
    fs::write(
        product.join("product-services.json"),
        r#"{
          "schemaVersion": 1,
          "marketplaceManager": {
            "metadataBaseUrl": "https://marketplace.zeta.example/metadata/",
            "targetsBaseUrl": "https://marketplace.zeta.example/targets/",
            "trustedRoot": "../root.json"
          }
        }"#,
    )
    .unwrap();

    assert!(
        LocalProductServicesConfig::load(
            product.join("product-services.json"),
            product.join("profile"),
        )
        .is_err()
    );
}
