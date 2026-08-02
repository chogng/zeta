use super::{ExplorerTree, ExplorerTreeAction};
use zeta_app_server_protocol::protocol::fs::{FsFileType, FsReadDirectoryEntry};
use zeta_ui::TreeItemExpansion;

#[test]
fn directory_activation_requests_children_once_and_preserves_their_identity() {
    let mut tree = ExplorerTree::default();
    tree.replace_root(vec![file("README.md"), directory("src")]);

    assert_eq!(tree.visible_len(), 2);
    assert_eq!(tree.row(0).unwrap().entry().label(), "src");
    assert_eq!(
        tree.visible_items()[0].expansion(),
        TreeItemExpansion::Collapsed
    );
    let directory_id = tree.row(0).unwrap().entry().element_id();

    assert_eq!(
        tree.activate_element(directory_id),
        Some(ExplorerTreeAction::LoadChildren {
            element: directory_id,
            path: "src".into(),
        })
    );
    assert!(tree.complete_directory_load(directory_id, vec![file("lib.rs")]));
    assert_eq!(tree.visible_len(), 3);
    assert_eq!(tree.row(1).unwrap().entry().label(), "lib.rs");
    assert_eq!(tree.row(1).unwrap().depth(), 1);
    assert_eq!(
        tree.visible_items()[0].expansion(),
        TreeItemExpansion::Expanded
    );
    let child_id = tree.row(1).unwrap().entry().element_id();

    assert_eq!(
        tree.activate_element(directory_id),
        Some(ExplorerTreeAction::StateChanged)
    );
    assert_eq!(tree.visible_len(), 2);
    assert_eq!(
        tree.activate_element(directory_id),
        Some(ExplorerTreeAction::StateChanged)
    );
    assert_eq!(tree.row(1).unwrap().entry().element_id(), child_id);
    assert_eq!(
        tree.navigate_right(directory_id),
        Some(ExplorerTreeAction::Focus(child_id))
    );
    assert_eq!(
        tree.navigate_left(child_id),
        Some(ExplorerTreeAction::Focus(directory_id))
    );
    assert_eq!(
        tree.navigate_left(directory_id),
        Some(ExplorerTreeAction::StateChanged)
    );
}

#[test]
fn app_server_entries_are_projected_without_client_side_filtering() {
    let mut tree = ExplorerTree::default();

    tree.replace_root(vec![directory("target"), file("alpha.txt")]);

    assert_eq!(tree.visible_len(), 2);
    assert_eq!(tree.row(0).unwrap().entry().label(), "target");
    assert_eq!(tree.row(1).unwrap().entry().label(), "alpha.txt");
    let file_id = tree.row(1).unwrap().entry().element_id();
    assert_eq!(
        tree.activate_element(file_id),
        Some(ExplorerTreeAction::OpenFile {
            path: "alpha.txt".into(),
        })
    );
}

fn directory(name: &str) -> FsReadDirectoryEntry {
    FsReadDirectoryEntry {
        name: name.into(),
        file_type: FsFileType::Directory,
    }
}

fn file(name: &str) -> FsReadDirectoryEntry {
    FsReadDirectoryEntry {
        name: name.into(),
        file_type: FsFileType::File,
    }
}
