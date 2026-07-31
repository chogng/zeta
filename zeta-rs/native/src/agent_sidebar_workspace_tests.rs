use std::sync::atomic::{AtomicU64, Ordering};

use super::AgentSidebarWorkspace;
use crate::workspace_context::WorkspaceContext;

static NEXT_WORKSPACE_ID: AtomicU64 = AtomicU64::new(0);

#[test]
fn replacing_workspace_rebuilds_the_files_root() {
    let fixture = std::env::temp_dir().join(format!(
        "zeta-agent-sidebar-workspace-{}-{}",
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
    let mut workspace = AgentSidebarWorkspace::new(&first_context);

    workspace.replace_workspace(&second_context);

    assert_eq!(workspace.root_entries().len(), 1);
    assert_eq!(workspace.root_entries()[0].label(), "second.txt");
    drop(workspace);
    std::fs::remove_dir_all(fixture).unwrap();
}
