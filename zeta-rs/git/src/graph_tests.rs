use std::num::NonZeroUsize;

use pretty_assertions::assert_eq;

use super::GitReferenceKind;
use crate::GitClient;
use crate::test_support::TestRepository;

#[tokio::test(flavor = "current_thread")]
async fn graph_includes_local_and_fetched_remote_refs() {
    let repository = TestRepository::init();
    repository.write("tracked.txt", "initial\n");
    repository.commit_all("initial");
    repository.git(&["branch", "topic"]);
    repository.git(&["switch", "topic"]);
    repository.write("tracked.txt", "topic\n");
    repository.commit_all("topic commit");
    let topic_oid = repository.git(&["rev-parse", "topic"]);
    repository.git(&["switch", "main"]);
    repository.git(&[
        "remote",
        "add",
        "origin",
        "https://github.com/example/zeta.git",
    ]);
    repository.git(&["update-ref", "refs/remotes/origin/topic", &topic_oid]);

    let client = GitClient::system();
    let opened = client
        .open_repository(repository.root())
        .await
        .expect("open repository");
    let graph = client
        .graph(&opened, NonZeroUsize::new(20).expect("non-zero"), 0)
        .await
        .expect("repository graph");

    assert!(graph.references().iter().any(|reference| {
        reference.name() == "main"
            && reference.kind() == GitReferenceKind::LocalBranch
            && reference.is_current()
    }));
    assert!(graph.references().iter().any(|reference| {
        reference.name() == "origin/topic"
            && reference.kind() == GitReferenceKind::RemoteBranch
            && reference.remote_name() == Some("origin")
            && reference.object_id() == topic_oid
    }));
    assert!(
        graph
            .commits()
            .iter()
            .any(|commit| commit.object_id() == topic_oid)
    );
    assert_eq!(graph.remotes().len(), 1);
    assert_eq!(
        graph.remotes()[0]
            .identity()
            .expect("remote identity")
            .repository(),
        "zeta"
    );
}

#[tokio::test(flavor = "current_thread")]
async fn graph_pages_commits_and_reports_more() {
    let repository = TestRepository::init();
    for (index, message) in ["first", "second", "third"].iter().enumerate() {
        repository.write("tracked.txt", &format!("{index}\n"));
        repository.commit_all(message);
    }

    let client = GitClient::system();
    let opened = client
        .open_repository(repository.root())
        .await
        .expect("open repository");
    let first_page = client
        .graph(&opened, NonZeroUsize::new(2).expect("non-zero"), 0)
        .await
        .expect("first graph page");
    let second_page = client
        .graph(&opened, NonZeroUsize::new(2).expect("non-zero"), 2)
        .await
        .expect("second graph page");

    assert_eq!(first_page.commits().len(), 2);
    assert!(first_page.has_more());
    assert_eq!(second_page.commits().len(), 1);
    assert!(!second_page.has_more());
    assert_eq!(first_page.commits()[0].subject(), "third");
    assert_eq!(first_page.commits()[1].subject(), "second");
    assert_eq!(second_page.commits()[0].subject(), "first");
}
