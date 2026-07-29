use super::FileInformation;
use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_DIRECTORY: AtomicU64 = AtomicU64::new(0);

struct TestDirectory(PathBuf);

impl TestDirectory {
    fn new() -> Self {
        let sequence = NEXT_DIRECTORY.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "zeta-file-identity-tests-{}-{sequence}",
            std::process::id()
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TestDirectory {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

#[test]
fn captures_stable_identity_for_the_same_file() {
    let directory = TestDirectory::new();
    let path = directory.path().join("same.txt");
    fs::write(&path, "same").unwrap();

    let from_path = FileInformation::from_path(&path).unwrap();
    let from_file = FileInformation::from_file(&File::open(&path).unwrap()).unwrap();

    assert_eq!(from_path.identity(), from_file.identity());
    assert_eq!(from_path.number_of_links(), 1);
}

#[test]
fn distinguishes_files_and_reports_hard_links() {
    let directory = TestDirectory::new();
    let first = directory.path().join("first.txt");
    let second = directory.path().join("second.txt");
    let linked = directory.path().join("linked.txt");
    fs::write(&first, "first").unwrap();
    fs::write(&second, "second").unwrap();
    fs::hard_link(&first, &linked).unwrap();

    let first_information = FileInformation::from_path(&first).unwrap();
    let linked_information = FileInformation::from_path(&linked).unwrap();
    let second_information = FileInformation::from_path(&second).unwrap();

    assert_eq!(first_information.identity(), linked_information.identity());
    assert_ne!(first_information.identity(), second_information.identity());
    assert_eq!(first_information.number_of_links(), 2);
}
