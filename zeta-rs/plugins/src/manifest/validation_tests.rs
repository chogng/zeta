use super::*;
use crate::PluginManifest;
use serde_json::{Value, json};

fn valid_manifest() -> Value {
    json!({
        "schemaVersion": 1,
        "id": "acme/code-review",
        "version": "1.2.3",
        "displayName": "Acme Code Review",
        "description": "Review workflows.",
        "license": "Apache-2.0",
        "compatibility": {
            "zeta": ">=0.1.0"
        },
        "contributions": {
            "skills": [
                { "id": "code-review", "path": "skills/code-review" }
            ],
            "mcpServers": [
                { "id": "review", "definition": "mcp/review-server.json" }
            ],
            "connectors": [
                {
                    "id": "review-account",
                    "displayName": "Acme Review",
                    "description": "Connect an Acme Review account.",
                    "mcpServer": "review"
                }
            ],
            "assets": [
                { "id": "icon", "path": "assets/icon.png" }
            ]
        },
        "permissions": [
            { "type": "process", "executable": "bin/review-server" },
            { "type": "workspace", "access": "read" },
            { "type": "network", "hosts": ["api.example.com"] }
        ],
        "credentialSlots": [
            {
                "name": "api-token",
                "kind": "secretText",
                "requiredFor": ["mcp:review", "connector:review-account"]
            }
        ],
        "metadata": {
            "acme/category": "review"
        }
    })
}

fn parse(value: &Value) -> Result<PluginManifest, crate::PluginError> {
    PluginManifest::from_json(&serde_json::to_vec(value).unwrap())
}

#[test]
fn strict_v1_manifest_parses_typed_security_fields() {
    let manifest = parse(&valid_manifest()).unwrap();

    assert_eq!(manifest.id.as_str(), "acme/code-review");
    assert_eq!(manifest.version.to_string(), "1.2.3");
    assert!(
        manifest
            .compatibility
            .zeta
            .matches(&semver::Version::new(0, 1, 0))
    );
    assert_eq!(manifest.contributions.skills[0].id.as_str(), "code-review");
    assert_eq!(
        manifest.credential_slots[0].required_for[0].to_string(),
        "mcp:review"
    );
    assert_eq!(
        manifest.credential_slots[0].required_for[1].to_string(),
        "connector:review-account"
    );
    assert!(matches!(
        manifest.permissions[2],
        crate::Permission::Network { .. }
    ));
}

#[test]
fn unknown_and_duplicate_json_fields_are_rejected() {
    let mut unknown = valid_manifest();
    unknown["autoApprove"] = json!(true);
    assert_eq!(
        parse(&unknown).unwrap_err().kind(),
        PluginErrorKind::ManifestInvalid
    );

    let duplicate = br#"{
        "schemaVersion": 1,
        "schemaVersion": 1,
        "id": "acme/review",
        "version": "1.0.0",
        "displayName": "Review",
        "compatibility": {"zeta": ">=0.1.0"},
        "contributions": {"assets": [{"id": "icon", "path": "assets/icon.png"}]}
    }"#;
    assert!(PluginManifest::from_json(duplicate).is_err());
}

#[test]
fn duplicate_contribution_and_credential_ids_are_rejected() {
    let mut duplicate = valid_manifest();
    duplicate["contributions"]["skills"] = json!([
        { "id": "review", "path": "skills/one" },
        { "id": "review", "path": "skills/two" }
    ]);
    assert!(serde_json::from_value::<PluginManifest>(duplicate.clone()).is_err());
    assert!(
        parse(&duplicate)
            .unwrap_err()
            .to_string()
            .contains("duplicate skill")
    );

    let mut slots = valid_manifest();
    slots["credentialSlots"] = json!([
        {"name": "token", "kind": "secretText", "requiredFor": []},
        {"name": "token", "kind": "secretText", "requiredFor": []}
    ]);
    assert!(
        parse(&slots)
            .unwrap_err()
            .to_string()
            .contains("duplicate credential slot")
    );
}

#[test]
fn credential_requirements_must_reference_declared_contributions() {
    let mut manifest = valid_manifest();
    manifest["credentialSlots"][0]["requiredFor"] = json!(["mcp:missing"]);

    let error = parse(&manifest).unwrap_err();

    assert!(error.to_string().contains("missing contribution"));
}

#[test]
fn connector_must_reference_a_declared_mcp_server() {
    let mut manifest = valid_manifest();
    manifest["contributions"]["connectors"][0]["mcpServer"] = json!("missing");

    let error = parse(&manifest).unwrap_err();

    assert!(error.to_string().contains("missing MCP server"));
}

#[test]
fn permissions_and_metadata_are_conservative_in_v1() {
    let mut wildcard = valid_manifest();
    wildcard["permissions"][2]["hosts"] = json!(["*.example.com"]);
    assert!(parse(&wildcard).is_err());

    let mut empty_network = valid_manifest();
    empty_network["permissions"][2]["hosts"] = json!([]);
    assert!(
        parse(&empty_network)
            .unwrap_err()
            .to_string()
            .contains("at least one exact host")
    );

    let mut metadata = valid_manifest();
    metadata["metadata"] = json!({"category": "review"});
    assert!(
        parse(&metadata)
            .unwrap_err()
            .to_string()
            .contains("namespaced")
    );
}

#[test]
fn schema_and_version_are_explicit() {
    let mut future = valid_manifest();
    future["schemaVersion"] = json!(2);
    assert!(
        parse(&future)
            .unwrap_err()
            .to_string()
            .contains("unsupported")
    );

    let mut non_semver = valid_manifest();
    non_semver["version"] = json!("latest");
    assert!(parse(&non_semver).is_err());
}
