use super::*;

#[test]
fn content_identities_validate_during_deserialization() {
    assert!(ContentDigest::new(format!("sha256:{}", "0".repeat(64))).is_ok());
    assert!(ContentDigest::new(format!("sha256:{}", "A".repeat(64))).is_err());
}
