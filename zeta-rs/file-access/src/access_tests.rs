use super::*;
use crate::GrantSource;

#[test]
#[cfg(unix)]
fn aliases_share_one_entry_with_independent_sources_and_revision() {
    let directory = tempfile::tempdir().unwrap();
    let alias_parent = tempfile::tempdir().unwrap();
    let alias = alias_parent.path().join("dir-alias");
    std::os::unix::fs::symlink(directory.path(), &alias).unwrap();
    let canonical = Dir::open_local(directory.path()).unwrap();
    let aliased = Dir::open_local(&alias).unwrap();
    let mut access = Access::new();

    assert_eq!(
        access
            .add(
                grant(canonical.clone(), file_permissions()),
                DirSource::PersistentConfiguration,
            )
            .unwrap(),
        Mutation::AddedDir
    );
    assert_eq!(
        access
            .add(
                grant(aliased, file_permissions()),
                DirSource::SessionRequest,
            )
            .unwrap(),
        Mutation::AddedSource
    );
    assert_eq!(access.revision().get(), 2);
    assert_eq!(access.dirs().len(), 1);
}

#[test]
fn snapshot_is_permission_bound_sorted_and_revoked_on_remove() {
    let zeta = Dir::open_local(tempfile::tempdir().unwrap().path()).unwrap();
    let alpha = Dir::open_local(tempfile::tempdir().unwrap().path()).unwrap();
    let mut access = Access::new();
    access
        .add(grant(zeta, file_permissions()), DirSource::SessionRequest)
        .unwrap();
    access
        .add(
            grant(alpha.clone(), file_permissions()),
            DirSource::SessionRequest,
        )
        .unwrap();

    let snapshot = access.snapshot(Permission::WriteFiles).unwrap();

    assert_eq!(snapshot.revision().get(), 2);
    assert_eq!(snapshot.authorizations().len(), 2);
    assert!(
        snapshot.authorizations()[0].dir().canonical_path()
            < snapshot.authorizations()[1].dir().canonical_path()
    );
    let alpha_authorization = snapshot
        .authorizations()
        .iter()
        .find(|authorization| authorization.dir() == &alpha)
        .unwrap()
        .clone();
    assert_eq!(
        access.remove(&alpha, DirSource::SessionRequest),
        Mutation::RemovedDir
    );
    assert!(alpha_authorization.ensure_active().is_err());
}

#[test]
fn idempotent_mutations_do_not_advance_revision() {
    let dir = Dir::open_local(tempfile::tempdir().unwrap().path()).unwrap();
    let mut access = Access::new();
    access
        .add(
            grant(dir.clone(), file_permissions()),
            DirSource::SessionRequest,
        )
        .unwrap();

    assert_eq!(
        access
            .add(
                grant(dir.clone(), file_permissions()),
                DirSource::SessionRequest,
            )
            .unwrap(),
        Mutation::AlreadyPresent
    );
    assert_eq!(
        access.remove(&dir, DirSource::LaunchArgument),
        Mutation::NotPresent
    );
    assert_eq!(access.revision().get(), 1);
}

#[test]
fn snapshots_include_only_dirs_granting_the_requested_permission() {
    let dir = Dir::open_local(tempfile::tempdir().unwrap().path()).unwrap();
    let mut access = Access::new();
    access
        .add(
            grant(dir.clone(), file_permissions()),
            DirSource::SessionRequest,
        )
        .unwrap();

    assert_eq!(
        access
            .snapshot(Permission::ReadFiles)
            .unwrap()
            .authorizations()
            .len(),
        1
    );
    let write_authorization = access
        .snapshot(Permission::WriteFiles)
        .unwrap()
        .authorizations()[0]
        .clone();
    assert_eq!(
        access
            .set_permissions(
                &dir,
                DirSource::SessionRequest,
                1,
                Permissions::new([Permission::ReadFiles]),
            )
            .unwrap(),
        Mutation::UpdatedPermissions
    );
    assert_eq!(access.revision().get(), 2);
    assert!(write_authorization.ensure_active().is_err());
    assert!(
        access
            .snapshot(Permission::WriteFiles)
            .unwrap()
            .authorizations()
            .is_empty()
    );
}

#[test]
fn dropping_access_revokes_existing_snapshots() {
    let authorization = {
        let dir = Dir::open_local(tempfile::tempdir().unwrap().path()).unwrap();
        let mut access = Access::new();
        access
            .add(grant(dir, file_permissions()), DirSource::SessionRequest)
            .unwrap();
        access
            .snapshot(Permission::ReadFiles)
            .unwrap()
            .authorizations()[0]
            .clone()
    };

    assert!(authorization.ensure_active().is_err());
}

#[test]
fn source_lifetime_controls_contributions() {
    let dir = Dir::open_local(tempfile::tempdir().unwrap().path()).unwrap();
    let mut access = Access::new();
    access
        .add(
            grant(
                dir.clone(),
                Permissions::new([
                    Permission::ReadFiles,
                    Permission::LoadInstructions,
                    Permission::DiscoverSkills,
                ]),
            ),
            DirSource::SessionRequest,
        )
        .unwrap();
    access
        .add(
            grant(dir.clone(), file_permissions()),
            DirSource::PersistentConfiguration,
        )
        .unwrap();

    assert!(access.contributions(&dir).allows(Contribution::Skills));
    assert!(
        access
            .contributions(&dir)
            .allows(Contribution::ProjectInstructions)
    );
    assert!(!access.contributions(&dir).allows(Contribution::McpServers));
    access.remove(&dir, DirSource::SessionRequest);
    assert_eq!(access.contributions(&dir).entries().len(), 0);
}

fn grant(dir: Dir, permissions: Permissions) -> Grant {
    Grant::for_environment(dir, GrantSource::ExplicitUser, permissions)
}

fn file_permissions() -> Permissions {
    Permissions::new([Permission::ReadFiles, Permission::WriteFiles])
}
