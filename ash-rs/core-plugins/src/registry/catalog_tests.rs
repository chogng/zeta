use semver::Version;

use super::CatalogManifest;
use super::language_ids_for;
use super::validate_manifest;

#[test]
fn accepts_signed_official_mcp_registry_provenance() {
    let manifest: CatalogManifest = serde_json::from_str(
        r#"{
          "schemaVersion": 2,
          "packageType": "mcp",
          "source": "thirdParty",
          "id": "marketplace/docs-mcp",
          "version": "1.2.3",
          "displayName": "Docs MCP",
          "description": "Search documentation.",
          "license": "MIT",
          "upstream": {
            "registry": "officialMcp",
            "name": "ac.example/docs-mcp",
            "version": "1.2.3",
            "recordUrl": "https://registry.modelcontextprotocol.io/v0.1/servers/ac.example%2Fdocs-mcp/versions/1.2.3",
            "repositoryUrl": "https://github.com/example/docs-mcp"
          },
          "capabilities": [{"kind": "mcp", "id": "docs-mcp", "path": "mcp/package.json"}]
        }"#,
    )
    .unwrap();

    validate_manifest(&manifest).unwrap();
    assert_eq!(manifest.upstream.unwrap().version, Version::new(1, 2, 3));
}

#[test]
fn rejects_registry_provenance_on_non_mcp_packages() {
    let manifest: CatalogManifest = serde_json::from_str(
        r#"{
          "schemaVersion": 2,
          "packageType": "skill",
          "source": "thirdParty",
          "id": "marketplace/docs-skill",
          "version": "1.2.3",
          "displayName": "Docs Skill",
          "description": "Search documentation.",
          "license": "MIT",
          "upstream": {
            "registry": "officialMcp",
            "name": "ac.example/docs-mcp",
            "version": "1.2.3",
            "recordUrl": "https://registry.modelcontextprotocol.io/v0.1/servers/ac.example%2Fdocs-mcp/versions/1.2.3"
          },
          "capabilities": [{"kind": "skill", "id": "docs", "path": "skill"}]
        }"#,
    )
    .unwrap();

    assert!(validate_manifest(&manifest).is_err());
}

#[test]
fn schema_two_language_routes_bind_exact_executable_capabilities() {
    let manifest: CatalogManifest = serde_json::from_str(
        r#"{
          "schemaVersion": 2,
          "packageType": "language",
          "source": "official",
          "id": "marketplace/demo-language",
          "version": "1.0.0",
          "displayName": "Demo Language",
          "description": "Demo language support.",
          "license": "MIT",
          "languages": [{
            "id": "demo",
            "displayName": "Demo",
            "aliases": ["demo"],
            "fileExtensions": [".demo"],
            "languageServer": "demo-server"
          }],
          "capabilities": [
            {"kind": "asset", "id": "language-assets", "path": "language"},
            {"kind": "executable", "id": "demo-server", "path": "server/demo.js", "runtime": "node"}
          ]
        }"#,
    )
    .unwrap();

    validate_manifest(&manifest).unwrap();
    let executable = manifest
        .capabilities
        .iter()
        .find(|capability| capability.id == "demo-server")
        .unwrap();
    assert_eq!(language_ids_for(&manifest, executable), vec!["demo"]);
}

#[test]
fn schema_two_language_rejects_an_unknown_server_route() {
    let manifest: CatalogManifest = serde_json::from_str(
        r#"{
          "schemaVersion": 2,
          "packageType": "language",
          "source": "official",
          "id": "marketplace/demo-language",
          "version": "1.0.0",
          "displayName": "Demo Language",
          "description": "Demo language support.",
          "license": "MIT",
          "languages": [{"id": "demo", "displayName": "Demo", "languageServer": "missing"}],
          "capabilities": [{"kind": "asset", "id": "language-assets", "path": "language"}]
        }"#,
    )
    .unwrap();

    assert!(validate_manifest(&manifest).is_err());
}
