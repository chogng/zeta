#[test]
fn reports_compiled_identity_consistently() {
    let info = super::BuildInfo::current();
    assert_eq!(info, super::BuildInfo::current());
    assert_eq!(info.version, super::VERSION);
    assert!(!info.target.is_empty());
    if info.commit.is_some() {
        assert!(info.build_id.is_some());
    }
    let json = serde_json::to_value(info).unwrap();
    assert!(json.get("buildId").is_some());
}
