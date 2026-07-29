use super::*;

#[test]
fn bundled_manifest_is_valid_and_preserves_seti_provenance() {
    let manifest = bundled_seti_manifest();

    assert!(manifest.version.contains("jesseweed/seti-ui/commit/"));
    assert_eq!(manifest.fonts[0].src[0].path, "./seti.woff");
    assert!(manifest.icon_definitions.contains_key("_rust"));
}

#[test]
fn parser_rejects_unknown_icon_references() {
    let mut fixture = serde_json::to_value(bundled_seti_manifest()).unwrap();
    fixture["file"] = serde_json::json!("_missing");

    let error = parse_seti_manifest(&serde_json::to_string(&fixture).unwrap()).unwrap_err();

    assert!(matches!(
        error,
        SetiManifestError::UnknownIconDefinition { icon_id, .. }
            if icon_id == "_missing"
    ));
}
