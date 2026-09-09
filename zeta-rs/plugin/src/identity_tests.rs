use super::*;

#[test]
fn plugin_ids_have_one_canonical_shape() {
    assert_eq!(
        PluginId::new("acme/code-review").unwrap().as_str(),
        "acme/code-review"
    );
    for invalid in [
        "code-review",
        "acme/",
        "/review",
        "Acme/review",
        "acme/code_review",
        "acme/-review",
        "acme/review-",
        "acme/code--review",
        "acme/review/extra",
    ] {
        assert_eq!(
            PluginId::new(invalid),
            Err(InvalidPluginId::InvalidShape),
            "{invalid}"
        );
    }
    assert_eq!(
        PluginId::new(format!("a/{}", "b".repeat(127))),
        Err(InvalidPluginId::TooLong)
    );
}

#[test]
fn plugin_versions_are_strict_semver() {
    assert_eq!(
        PluginVersion::new("1.2.3-alpha.1+build.4")
            .unwrap()
            .to_string(),
        "1.2.3-alpha.1+build.4"
    );
    assert_eq!(
        PluginVersion::new("1.2"),
        Err(InvalidPluginVersion::InvalidSemver)
    );
    assert_eq!(
        PluginVersion::new("v1.2.3"),
        Err(InvalidPluginVersion::InvalidSemver)
    );
}

#[test]
fn package_digests_are_self_describing_and_canonical() {
    let digest = PluginPackageDigest::sha256(b"package");
    assert!(digest.as_str().starts_with("sha256:"));
    assert_eq!(digest.as_str().len(), 7 + 64);
    assert_eq!(
        PluginPackageDigest::new(digest.to_string()).unwrap(),
        digest
    );
    assert!(PluginPackageDigest::new("abc").is_err());
    assert!(PluginPackageDigest::new(format!("sha256:{}", "A".repeat(64))).is_err());
}

#[test]
fn identity_types_validate_during_deserialization() {
    assert!(serde_json::from_str::<PluginId>("\"Acme/review\"").is_err());
    assert!(serde_json::from_str::<PluginVersion>("\"latest\"").is_err());
    assert!(
        serde_json::from_str::<PluginPackageDigest>(&format!("\"sha256:{}\"", "0".repeat(64)))
            .is_ok()
    );
}
