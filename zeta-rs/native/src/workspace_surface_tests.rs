use super::WorkspaceSurface;

#[test]
fn agent_is_default_and_terminal_is_an_explicit_reversible_surface() {
    let mut surface = WorkspaceSurface::default();

    assert_eq!(surface, WorkspaceSurface::Agent);
    assert!(!surface.is_terminal());
    surface.toggle();
    assert_eq!(surface, WorkspaceSurface::Terminal);
    assert!(surface.is_terminal());
    surface.toggle();
    assert_eq!(surface, WorkspaceSurface::Agent);
}
