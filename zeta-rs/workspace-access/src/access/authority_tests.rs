use super::*;
use crate::AdditionalDirectoryContribution;
use crate::AdditionalDirectoryContributionPolicy;
use crate::AdditionalInstructionsPolicy;
use zeta_workspace::WorkspaceTrustDecision;
use zeta_workspace::WorkspaceTrustSource;

#[test]
fn working_directory_cannot_be_added_as_an_additional_directory() {
    let directory = tempfile::tempdir().unwrap();
    let root = WorkspaceRoot::open(directory.path()).unwrap();
    let mut authority = WorkspaceAccessAuthority::new(root.clone());

    assert_eq!(
        authority.add_directory(
            authorization(root),
            AdditionalDirectorySource::SessionCommand
        ),
        Err(WorkspaceAccessError::WorkingDirectoryCannotBeAdditional)
    );
}

#[test]
#[cfg(unix)]
fn canonical_aliases_share_one_entry_with_independent_sources_and_revision() {
    let working = tempfile::tempdir().unwrap();
    let additional = tempfile::tempdir().unwrap();
    let alias_parent = tempfile::tempdir().unwrap();
    let alias = alias_parent.path().join("additional-alias");
    std::os::unix::fs::symlink(additional.path(), &alias).unwrap();
    let mut authority = WorkspaceAccessAuthority::new(WorkspaceRoot::open(working.path()).unwrap());
    let canonical = WorkspaceRoot::open(additional.path()).unwrap();
    let aliased = WorkspaceRoot::open(&alias).unwrap();

    assert_eq!(
        authority
            .add_directory(
                authorization(canonical.clone()),
                AdditionalDirectorySource::PersistentConfiguration,
            )
            .unwrap(),
        WorkspaceAccessMutation::AddedDirectory
    );
    assert_eq!(
        authority
            .add_directory(
                authorization(aliased),
                AdditionalDirectorySource::SessionCommand,
            )
            .unwrap(),
        WorkspaceAccessMutation::AddedSource
    );
    assert_eq!(authority.revision().get(), 2);
    assert_eq!(authority.additional_directories().len(), 1);
}

#[test]
fn snapshot_is_capability_bound_sorted_and_revoked_on_remove() {
    let working = tempfile::tempdir().unwrap();
    let zeta = tempfile::tempdir().unwrap();
    let alpha = tempfile::tempdir().unwrap();
    let mut authority = WorkspaceAccessAuthority::new(WorkspaceRoot::open(working.path()).unwrap());
    let zeta_root = WorkspaceRoot::open(zeta.path()).unwrap();
    let alpha_root = WorkspaceRoot::open(alpha.path()).unwrap();
    authority
        .add_directory(
            authorization(zeta_root.clone()),
            AdditionalDirectorySource::SessionCommand,
        )
        .unwrap();
    authority
        .add_directory(
            authorization(alpha_root.clone()),
            AdditionalDirectorySource::SessionCommand,
        )
        .unwrap();

    let snapshot = authority
        .snapshot_for(WorkspaceCapability::MutateRepository)
        .unwrap();

    assert_eq!(snapshot.revision().get(), 2);
    assert_eq!(snapshot.additional_roots().len(), 2);
    assert!(
        snapshot.additional_roots()[0].root().canonical_path()
            < snapshot.additional_roots()[1].root().canonical_path()
    );
    let alpha_token = snapshot
        .additional_roots()
        .iter()
        .find(|token| token.root() == &alpha_root)
        .unwrap()
        .clone();
    assert_eq!(
        authority.remove_directory(&alpha_root, AdditionalDirectorySource::SessionCommand),
        WorkspaceAccessMutation::RemovedDirectory
    );
    assert!(alpha_token.ensure_active().is_err());
}

#[test]
fn idempotent_mutations_do_not_advance_revision() {
    let working = tempfile::tempdir().unwrap();
    let additional = tempfile::tempdir().unwrap();
    let root = WorkspaceRoot::open(additional.path()).unwrap();
    let mut authority = WorkspaceAccessAuthority::new(WorkspaceRoot::open(working.path()).unwrap());
    authority
        .add_directory(
            authorization(root.clone()),
            AdditionalDirectorySource::SessionCommand,
        )
        .unwrap();

    assert_eq!(
        authority
            .add_directory(
                authorization(root.clone()),
                AdditionalDirectorySource::SessionCommand,
            )
            .unwrap(),
        WorkspaceAccessMutation::AlreadyPresent
    );
    assert_eq!(
        authority.remove_directory(&root, AdditionalDirectorySource::LaunchArgument),
        WorkspaceAccessMutation::NotPresent
    );
    assert_eq!(authority.revision().get(), 1);
}

#[test]
fn source_lifetime_controls_contribution_policy() {
    let working = tempfile::tempdir().unwrap();
    let additional = tempfile::tempdir().unwrap();
    let root = WorkspaceRoot::open(additional.path()).unwrap();
    let mut authority = WorkspaceAccessAuthority::new(WorkspaceRoot::open(working.path()).unwrap());
    authority
        .add_directory(
            authorization(root.clone()),
            AdditionalDirectorySource::SessionCommand,
        )
        .unwrap();
    authority
        .add_directory(
            authorization(root.clone()),
            AdditionalDirectorySource::PersistentConfiguration,
        )
        .unwrap();
    let directory = &authority.additional_directories()[0];

    assert!(
        directory
            .contribution_policy(AdditionalInstructionsPolicy::Exclude)
            .allows(AdditionalDirectoryContribution::Skills)
    );
    authority.remove_directory(&root, AdditionalDirectorySource::SessionCommand);
    assert_eq!(
        authority.additional_directories()[0]
            .contribution_policy(AdditionalInstructionsPolicy::Include),
        AdditionalDirectoryContributionPolicy::FileAccessOnly
    );
}

fn authorization(root: WorkspaceRoot) -> WorkspaceAuthorization {
    WorkspaceAuthorization::new(
        root,
        WorkspaceTrustDecision::Trusted(WorkspaceTrustSource::ExplicitUserDecision),
    )
}
