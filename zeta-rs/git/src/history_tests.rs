use std::path::Path;

use pretty_assertions::assert_eq;

use crate::GitChangeStatus;
use crate::GitClient;
use crate::test_support::TestRepository;

#[tokio::test]
async fn commit_changes_and_content_follow_the_first_parent() {
    let repository = TestRepository::init();
    repository.write("modified.txt", "before\n");
    repository.write("renamed.txt", "rename me\n");
    repository.commit_all("initial");
    repository.write("modified.txt", "after\n");
    repository.git(&["mv", "renamed.txt", "moved.txt"]);
    repository.write("added.txt", "added\n");
    repository.commit_all("change files");
    let object_id = repository.git(&["rev-parse", "HEAD"]);

    let client = GitClient::system();
    let opened = client.open_repository(repository.root()).await.unwrap();
    let (parent, changes) = client.commit_changes(&opened, &object_id).await.unwrap();

    assert!(parent.is_some());
    assert!(changes.iter().any(|change| {
        change.path() == Path::new("modified.txt") && change.status() == GitChangeStatus::Modified
    }));
    let renamed = changes
        .iter()
        .find(|change| change.path() == Path::new("moved.txt"))
        .expect("renamed change");
    assert_eq!(renamed.original_path(), Some(Path::new("renamed.txt")));
    assert_eq!(renamed.status(), GitChangeStatus::Renamed);

    let content = client
        .commit_file(
            &opened,
            &object_id,
            parent.as_deref(),
            Path::new("modified.txt"),
            None,
            1024,
        )
        .await
        .unwrap();
    assert_eq!(content.original(), Some(b"before\n".as_slice()));
    assert_eq!(content.modified(), Some(b"after\n".as_slice()));
}

#[tokio::test]
async fn root_commit_changes_have_no_parent_or_original_content() {
    let repository = TestRepository::init();
    repository.write("first.txt", "first\n");
    repository.commit_all("root");
    let object_id = repository.git(&["rev-parse", "HEAD"]);

    let client = GitClient::system();
    let opened = client.open_repository(repository.root()).await.unwrap();
    let (parent, changes) = client.commit_changes(&opened, &object_id).await.unwrap();
    assert_eq!(parent, None);
    assert_eq!(changes.len(), 1);
    assert_eq!(changes[0].status(), GitChangeStatus::Added);

    let content = client
        .commit_file(
            &opened,
            &object_id,
            None,
            Path::new("first.txt"),
            None,
            1024,
        )
        .await
        .unwrap();
    assert_eq!(content.original(), None);
    assert_eq!(content.modified(), Some(b"first\n".as_slice()));
}
