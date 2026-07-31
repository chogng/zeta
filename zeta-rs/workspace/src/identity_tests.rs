use super::*;
use crate::WorkspaceRoot;

#[test]
#[cfg(unix)]
fn aliases_share_one_trust_identity() {
    let directory = tempfile::tempdir().unwrap();
    let alias_parent = tempfile::tempdir().unwrap();
    let alias = alias_parent.path().join("workspace-alias");
    std::os::unix::fs::symlink(directory.path(), &alias).unwrap();

    let canonical = WorkspaceRoot::open(directory.path()).unwrap();
    let aliased = WorkspaceRoot::open(&alias).unwrap();

    assert_eq!(canonical.trust_id(), aliased.trust_id());
}

#[test]
fn serialized_identity_is_validated_and_normalized() {
    let uppercase = format!("sha256:{}", "AB".repeat(32));
    let identity: WorkspaceTrustId = uppercase.parse().unwrap();

    assert_eq!(identity.as_str(), format!("sha256:{}", "ab".repeat(32)));
    assert!("sha256:not-a-digest".parse::<WorkspaceTrustId>().is_err());
}
