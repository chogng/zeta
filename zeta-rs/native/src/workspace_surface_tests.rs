use super::{WorkspaceSurface, WorkspaceSurfaceKind};

#[test]
fn agent_is_default_and_terminal_is_an_explicit_reversible_surface() {
    let mut surface = WorkspaceSurface::default();

    assert_eq!(surface.active(), WorkspaceSurfaceKind::Agent);
    assert!(!surface.is_terminal());
    surface.toggle_terminal();
    assert_eq!(surface.active(), WorkspaceSurfaceKind::Terminal);
    assert!(surface.is_terminal());
    surface.toggle_terminal();
    assert_eq!(surface.active(), WorkspaceSurfaceKind::Agent);
}

#[test]
fn terminal_returns_to_the_file_editor_that_opened_it() {
    let mut surface = WorkspaceSurface::default();

    surface.show_editor();
    assert_eq!(surface.active(), WorkspaceSurfaceKind::Editor);
    surface.toggle_terminal();
    assert_eq!(surface.active(), WorkspaceSurfaceKind::Terminal);
    surface.toggle_terminal();
    assert_eq!(surface.active(), WorkspaceSurfaceKind::Editor);

    surface.show_agent();
    assert_eq!(surface.active(), WorkspaceSurfaceKind::Agent);
}
