use std::sync::atomic::{AtomicU64, Ordering};

use super::{ExplorerTree, ExplorerTreeNavigation};
use zeta_ui::TreeItemExpansion;

static NEXT_TREE_ID: AtomicU64 = AtomicU64::new(0);

#[test]
fn directory_activation_lazily_projects_sorted_children_and_collapses_them() {
    let fixture = std::env::temp_dir().join(format!(
        "zeta-explorer-tree-{}-{}",
        std::process::id(),
        NEXT_TREE_ID.fetch_add(1, Ordering::Relaxed)
    ));
    let source = fixture.join("src");
    std::fs::create_dir_all(&source).unwrap();
    std::fs::write(source.join("lib.rs"), "lib").unwrap();
    std::fs::write(fixture.join("README.md"), "readme").unwrap();
    let mut tree = ExplorerTree::default();
    tree.replace_root(Some(&fixture));

    assert_eq!(tree.visible_len(), 2);
    assert_eq!(tree.row(0).unwrap().entry().label(), "src");
    assert_eq!(
        tree.visible_items()[0].expansion(),
        TreeItemExpansion::Collapsed
    );
    let directory_id = tree.row(0).unwrap().entry().element_id();

    assert!(tree.activate_element(directory_id));
    assert_eq!(tree.visible_len(), 3);
    assert_eq!(tree.row(1).unwrap().entry().label(), "lib.rs");
    assert_eq!(tree.row(1).unwrap().depth(), 1);
    assert_eq!(
        tree.visible_items()[0].expansion(),
        TreeItemExpansion::Expanded
    );
    let child_id = tree.row(1).unwrap().entry().element_id();

    assert!(tree.activate_element(directory_id));
    assert_eq!(tree.visible_len(), 2);
    assert_eq!(
        tree.visible_items()[0].expansion(),
        TreeItemExpansion::Collapsed
    );
    assert!(tree.activate_element(directory_id));
    assert_eq!(tree.row(1).unwrap().entry().element_id(), child_id);
    assert_eq!(
        tree.navigate_right(directory_id),
        Some(ExplorerTreeNavigation::Focus(child_id))
    );
    assert_eq!(
        tree.navigate_left(child_id),
        Some(ExplorerTreeNavigation::Focus(directory_id))
    );
    assert_eq!(
        tree.navigate_left(directory_id),
        Some(ExplorerTreeNavigation::StateChanged)
    );

    std::fs::remove_dir_all(fixture).unwrap();
}

#[test]
fn ignored_workspace_directories_are_not_projected() {
    let fixture = std::env::temp_dir().join(format!(
        "zeta-explorer-tree-ignore-{}-{}",
        std::process::id(),
        NEXT_TREE_ID.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir_all(fixture.join("target")).unwrap();
    std::fs::write(fixture.join("alpha.txt"), "alpha").unwrap();
    let mut tree = ExplorerTree::default();
    tree.replace_root(Some(&fixture));

    assert_eq!(tree.visible_len(), 1);
    assert_eq!(tree.row(0).unwrap().entry().label(), "alpha.txt");
    std::fs::remove_dir_all(fixture).unwrap();
}
