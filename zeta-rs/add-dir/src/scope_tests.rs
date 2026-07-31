use super::*;
use crate::AdditionalDirectoryContribution::{
    EnabledPlugins, InstructionRules, ProjectInstructions, Skills,
};

#[test]
fn working_directory_cannot_be_added_as_an_additional_directory() {
    let directory = tempfile::tempdir().unwrap();
    let root = WorkspaceRoot::open(directory.path()).unwrap();
    let mut scope = DirectoryAccessScope::new(root.clone());

    assert_eq!(
        scope.add_directory(root, AdditionalDirectorySource::SessionCommand),
        Err(DirectoryScopeError::WorkingDirectoryCannotBeAdditional)
    );
}

#[test]
#[cfg(unix)]
fn canonical_aliases_share_one_entry_with_independent_sources() {
    let working = tempfile::tempdir().unwrap();
    let additional = tempfile::tempdir().unwrap();
    let alias_parent = tempfile::tempdir().unwrap();
    let alias = alias_parent.path().join("additional-alias");
    create_directory_symlink(additional.path(), &alias);
    let mut scope = DirectoryAccessScope::new(WorkspaceRoot::open(working.path()).unwrap());
    let canonical = WorkspaceRoot::open(additional.path()).unwrap();
    let aliased = WorkspaceRoot::open(&alias).unwrap();

    assert_eq!(
        scope
            .add_directory(
                canonical.clone(),
                AdditionalDirectorySource::PersistentConfiguration,
            )
            .unwrap(),
        DirectoryScopeMutation::AddedDirectory
    );
    assert_eq!(
        scope
            .add_directory(aliased, AdditionalDirectorySource::SessionCommand)
            .unwrap(),
        DirectoryScopeMutation::AddedSource
    );
    assert_eq!(scope.additional_directories().len(), 1);
    assert_eq!(
        scope.additional_directories()[0].sources(),
        [
            AdditionalDirectorySource::SessionCommand,
            AdditionalDirectorySource::PersistentConfiguration,
        ]
    );
}

#[test]
fn persistent_configuration_is_file_access_only() {
    let working = tempfile::tempdir().unwrap();
    let additional = tempfile::tempdir().unwrap();
    let additional_root = WorkspaceRoot::open(additional.path()).unwrap();
    let mut scope = DirectoryAccessScope::new(WorkspaceRoot::open(working.path()).unwrap());
    scope
        .add_directory(
            additional_root,
            AdditionalDirectorySource::PersistentConfiguration,
        )
        .unwrap();

    let policy = scope.additional_directories()[0]
        .contribution_policy(AdditionalInstructionsPolicy::Include);

    assert_eq!(
        policy,
        AdditionalDirectoryContributionPolicy::FileAccessOnly
    );
    assert!(policy.contributions().is_empty());
}

#[test]
fn transient_sources_expose_only_named_contributions() {
    let working = tempfile::tempdir().unwrap();
    let additional = tempfile::tempdir().unwrap();
    let mut scope = DirectoryAccessScope::new(WorkspaceRoot::open(working.path()).unwrap());
    scope
        .add_directory(
            WorkspaceRoot::open(additional.path()).unwrap(),
            AdditionalDirectorySource::LaunchArgument,
        )
        .unwrap();
    let directory = &scope.additional_directories()[0];

    let default_policy = directory.contribution_policy(AdditionalInstructionsPolicy::Exclude);
    assert!(default_policy.allows(Skills));
    assert!(default_policy.allows(EnabledPlugins));
    assert!(!default_policy.allows(ProjectInstructions));

    let instruction_policy = directory.contribution_policy(AdditionalInstructionsPolicy::Include);
    assert!(instruction_policy.allows(ProjectInstructions));
    assert!(instruction_policy.allows(InstructionRules));
}

#[test]
fn removing_one_source_preserves_other_access_and_recomputes_policy() {
    let working = tempfile::tempdir().unwrap();
    let additional = tempfile::tempdir().unwrap();
    let additional_root = WorkspaceRoot::open(additional.path()).unwrap();
    let mut scope = DirectoryAccessScope::new(WorkspaceRoot::open(working.path()).unwrap());
    scope
        .add_directory(
            additional_root.clone(),
            AdditionalDirectorySource::SessionCommand,
        )
        .unwrap();
    scope
        .add_directory(
            additional_root.clone(),
            AdditionalDirectorySource::PersistentConfiguration,
        )
        .unwrap();

    assert_eq!(
        scope.remove_directory(&additional_root, AdditionalDirectorySource::SessionCommand,),
        DirectoryScopeMutation::RemovedSource
    );
    assert_eq!(scope.additional_directories().len(), 1);
    assert_eq!(
        scope.additional_directories()[0]
            .contribution_policy(AdditionalInstructionsPolicy::Include),
        AdditionalDirectoryContributionPolicy::FileAccessOnly
    );
    assert_eq!(
        scope.remove_directory(
            &additional_root,
            AdditionalDirectorySource::PersistentConfiguration,
        ),
        DirectoryScopeMutation::RemovedDirectory
    );
    assert!(scope.additional_directories().is_empty());
}

#[test]
fn repeated_add_and_missing_remove_are_idempotent() {
    let working = tempfile::tempdir().unwrap();
    let additional = tempfile::tempdir().unwrap();
    let additional_root = WorkspaceRoot::open(additional.path()).unwrap();
    let mut scope = DirectoryAccessScope::new(WorkspaceRoot::open(working.path()).unwrap());

    assert_eq!(
        scope
            .add_directory(
                additional_root.clone(),
                AdditionalDirectorySource::SessionCommand,
            )
            .unwrap(),
        DirectoryScopeMutation::AddedDirectory
    );
    assert_eq!(
        scope
            .add_directory(
                additional_root.clone(),
                AdditionalDirectorySource::SessionCommand,
            )
            .unwrap(),
        DirectoryScopeMutation::AlreadyPresent
    );
    assert_eq!(
        scope.remove_directory(&additional_root, AdditionalDirectorySource::LaunchArgument,),
        DirectoryScopeMutation::NotPresent
    );
}

#[cfg(unix)]
fn create_directory_symlink(target: &std::path::Path, link: &std::path::Path) {
    std::os::unix::fs::symlink(target, link).unwrap();
}
