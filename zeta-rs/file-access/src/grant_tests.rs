use super::*;

#[test]
fn grant_issues_only_explicit_permissions() {
    let directory = tempfile::tempdir().unwrap();
    let dir = Dir::open_local(directory.path()).unwrap();
    let grant = Grant::for_environment(
        dir,
        GrantSource::ExplicitUser,
        Permissions::new([Permission::ReadFiles]),
    );

    assert!(grant.authorize(Permission::ReadFiles).is_ok());
    assert!(grant.authorize(Permission::ExecuteCommands).is_err());
}

#[test]
fn authorization_retains_the_checked_permission_and_source() {
    let directory = tempfile::tempdir().unwrap();
    let subject = GrantSubject::SessionTree(zeta_protocol::SessionId::new("session-1").unwrap());
    let authorization = Authorization::evaluate(
        subject.clone(),
        Dir::open_local(directory.path()).unwrap(),
        GrantSource::ExplicitUser,
        Permissions::new([Permission::ExecuteCommands]),
        Permission::ExecuteCommands,
    )
    .unwrap();

    assert_eq!(authorization.permission(), Permission::ExecuteCommands);
    assert_eq!(authorization.source(), GrantSource::ExplicitUser);
    assert_eq!(authorization.subject(), &subject);
}

#[test]
fn revocation_invalidates_existing_authorizations() {
    let directory = tempfile::tempdir().unwrap();
    let grant = Grant::for_environment(
        Dir::open_local(directory.path()).unwrap(),
        GrantSource::OrganizationPolicy,
        Permissions::new([Permission::WriteFiles]),
    );
    let authorization = grant.authorize(Permission::WriteFiles).unwrap();

    grant.revoke();

    assert!(!grant.is_active());
    assert!(authorization.ensure_active().is_err());
}
