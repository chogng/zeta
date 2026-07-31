use super::*;

#[test]
fn restricted_workspace_cannot_acquire_execution_capability() {
    let directory = tempfile::tempdir().unwrap();
    let root = WorkspaceRoot::open(directory.path()).unwrap();

    let error = TrustedWorkspace::require(
        root.clone(),
        WorkspaceTrustDecision::Restricted,
        WorkspaceCapability::ExecuteProcess,
    )
    .unwrap_err();

    assert_eq!(error.root(), &root);
    assert_eq!(error.capability(), WorkspaceCapability::ExecuteProcess);
}

#[test]
fn trusted_token_remains_bound_to_the_approved_root() {
    let first = tempfile::tempdir().unwrap();
    let second = tempfile::tempdir().unwrap();
    let first = WorkspaceRoot::open(first.path()).unwrap();
    let second = WorkspaceRoot::open(second.path()).unwrap();

    let trusted = TrustedWorkspace::require(
        first.clone(),
        WorkspaceTrustDecision::Trusted(WorkspaceTrustSource::ExplicitUserDecision),
        WorkspaceCapability::ExecuteProcess,
    )
    .unwrap();

    assert_eq!(trusted.root(), &first);
    assert_ne!(trusted.root(), &second);
    assert_eq!(trusted.source(), WorkspaceTrustSource::ExplicitUserDecision);
    assert_eq!(trusted.capability(), WorkspaceCapability::ExecuteProcess);
}

#[test]
fn revocation_invalidates_existing_and_future_capability_tokens() {
    let directory = tempfile::tempdir().unwrap();
    let root = WorkspaceRoot::open(directory.path()).unwrap();
    let authorization = WorkspaceAuthorization::new(
        root,
        WorkspaceTrustDecision::Trusted(WorkspaceTrustSource::ExplicitUserDecision),
    );
    let token = authorization
        .require(WorkspaceCapability::ExecuteProcess)
        .unwrap();

    authorization.revoke();

    assert!(!authorization.is_active());
    assert!(token.ensure_active().is_err());
    assert!(
        authorization
            .require(WorkspaceCapability::MutateRepository)
            .is_err()
    );
}
