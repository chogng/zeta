use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;

use zui::ui::Size;

use super::DirectoryEntry;
use super::FilesState;

static NEXT_DIR_ID: AtomicU64 = AtomicU64::new(0);

#[test]
fn replacing_dir_rebuilds_the_files_root() {
    let fixture = test_dir("replace");
    let first = fixture.join("first");
    let second = fixture.join("second");
    std::fs::create_dir_all(&first).unwrap();
    std::fs::create_dir_all(&second).unwrap();
    let mut files = FilesState::default();
    files.set_dir_root(first);
    files.refresh(vec![DirectoryEntry::file("first.txt")]);

    files.set_dir_root(second);
    files.refresh(vec![DirectoryEntry::file("second.txt")]);

    assert_eq!(files.tree_row(0).unwrap().entry().label(), "second.txt");
    std::fs::remove_dir_all(fixture).unwrap();
}

#[test]
fn refreshing_files_resets_pixel_scroll() {
    let fixture = test_dir("scroll");
    std::fs::create_dir_all(&fixture).unwrap();
    let mut files = FilesState::default();
    files.set_dir_root(fixture.clone());
    files.refresh(
        (0..20)
            .map(|index| DirectoryEntry::file(format!("file-{index:02}.txt")))
            .collect(),
    );
    assert!(files.scroll(72.0, Size::new(320.0, 100.0)));
    assert_eq!(files.scroll_state().vertical_offset(), 72.0);

    files.refresh(Vec::new());

    assert_eq!(files.scroll_state().vertical_offset(), 0.0);
    std::fs::remove_dir_all(fixture).unwrap();
}

fn test_dir(label: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!(
        "zeta-files-{label}-{}-{}",
        std::process::id(),
        NEXT_DIR_ID.fetch_add(1, Ordering::Relaxed)
    ))
}
