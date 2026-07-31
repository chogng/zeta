use std::path::Path;

use super::GitFileRevision;
use crate::GitClient;
use crate::test_support::TestRepository;

#[tokio::test]
async fn reads_head_and_index_content_and_reports_missing_paths() {
    let repository = TestRepository::init();
    repository.write("tracked.txt", "head\n");
    repository.git(&["add", "tracked.txt"]);
    repository.git(&["commit", "-m", "initial"]);
    repository.write("tracked.txt", "index\n");
    repository.git(&["add", "tracked.txt"]);

    let client = GitClient::system();
    let opened = client.open_repository(repository.root()).await.unwrap();

    assert_eq!(
        client
            .read_file_at_revision(
                &opened,
                Path::new("tracked.txt"),
                GitFileRevision::Head,
                1024,
            )
            .await
            .unwrap(),
        Some(b"head\n".to_vec())
    );
    assert_eq!(
        client
            .read_file_at_revision(
                &opened,
                Path::new("tracked.txt"),
                GitFileRevision::Index,
                1024,
            )
            .await
            .unwrap(),
        Some(b"index\n".to_vec())
    );
    assert_eq!(
        client
            .read_file_at_revision(
                &opened,
                Path::new("missing.txt"),
                GitFileRevision::Head,
                1024,
            )
            .await
            .unwrap(),
        None
    );
}
