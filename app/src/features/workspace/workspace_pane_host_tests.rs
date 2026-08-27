use std::sync::atomic::{AtomicU64, Ordering};

use super::WorkspacePaneHost;
use crate::workspace_context::WorkspaceContext;
use zeta_app_server_protocol::protocol::fs::{FsFileType, FsReadDirectoryEntry};
use zui::ui::Size;

static NEXT_WORKSPACE_ID: AtomicU64 = AtomicU64::new(0);

#[test]
fn replacing_workspace_rebuilds_the_files_root() {
    let fixture = std::env::temp_dir().join(format!(
        "zeta-workspace-pane-workspace-{}-{}",
        std::process::id(),
        NEXT_WORKSPACE_ID.fetch_add(1, Ordering::Relaxed)
    ));
    let first = fixture.join("first");
    let second = fixture.join("second");
    std::fs::create_dir_all(&first).unwrap();
    std::fs::create_dir_all(&second).unwrap();
    std::fs::write(first.join("first.txt"), "first").unwrap();
    std::fs::write(second.join("second.txt"), "second").unwrap();
    let mut first_context = WorkspaceContext::capture_current();
    first_context
        .switch_working_directory(first.clone())
        .unwrap();
    let mut second_context = WorkspaceContext::capture_current();
    second_context
        .switch_working_directory(second.clone())
        .unwrap();
    let mut workspace = WorkspacePaneHost::new(&first_context);

    workspace.replace_workspace(&second_context);
    workspace.refresh_files(vec![file("second.txt")]);

    assert_eq!(
        workspace.file_tree_row(0).unwrap().entry().label(),
        "second.txt"
    );
    drop(workspace);
    std::fs::remove_dir_all(fixture).unwrap();
}

#[test]
fn file_list_retains_pixel_scroll_and_resets_it_when_files_refresh() {
    let fixture = std::env::temp_dir().join(format!(
        "zeta-workspace-pane-scroll-{}-{}",
        std::process::id(),
        NEXT_WORKSPACE_ID.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&fixture).unwrap();
    for index in 0..20 {
        std::fs::write(
            fixture.join(format!("file-{index:02}.txt")),
            index.to_string(),
        )
        .unwrap();
    }
    let mut context = WorkspaceContext::capture_current();
    context.switch_working_directory(fixture.clone()).unwrap();
    let mut workspace = WorkspacePaneHost::new(&context);
    workspace.refresh_files(
        (0..20)
            .map(|index| file(&format!("file-{index:02}.txt")))
            .collect(),
    );

    assert!(workspace.scroll_file_list(72.0, Size::new(320.0, 100.0)));
    assert_eq!(workspace.file_list_scroll_state().vertical_offset(), 72.0);

    workspace.refresh_files(Vec::new());

    assert_eq!(workspace.file_list_scroll_state().vertical_offset(), 0.0);
    drop(workspace);
    std::fs::remove_dir_all(fixture).unwrap();
}

fn file(name: &str) -> FsReadDirectoryEntry {
    FsReadDirectoryEntry {
        name: name.into(),
        file_type: FsFileType::File,
    }
}
