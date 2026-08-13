use crate::RemoteMarketplaceErrorKind;
use crate::metadata::RevocationDocument;

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
