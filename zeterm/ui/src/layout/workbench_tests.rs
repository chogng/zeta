use super::SessionSidebarLayoutSpec;
use super::SidebarLayoutSpec;
use super::WorkbenchLayoutSpec;
use super::WorkbenchPart;
use crate::layout::LogicalViewport;
use crate::layout::SidebarVisibility;
use zui::ui::Rect;

fn sessions(visibility: SidebarVisibility) -> SessionSidebarLayoutSpec {
    SessionSidebarLayoutSpec::new(visibility, 200.0, 160.0, 480.0, 240.0)
}

fn inspector(visibility: SidebarVisibility) -> SidebarLayoutSpec {
    SidebarLayoutSpec::new(visibility, 320.0, 240.0, 560.0, 240.0)
}

fn spec(
    sessions_visibility: SidebarVisibility,
    inspector_visibility: SidebarVisibility,
) -> WorkbenchLayoutSpec {
    WorkbenchLayoutSpec::new(
        32.0,
        sessions(sessions_visibility),
        inspector(inspector_visibility),
    )
}

#[test]
fn workbench_composes_titlebar_sessions_main_and_inspector_parts() {
    let layout = spec(SidebarVisibility::Expanded, SidebarVisibility::Expanded)
        .for_viewport(LogicalViewport {
            width: 1_000.0,
            height: 700.0,
        })
        .expect("viewport should fit the Workbench");

    assert_eq!(
        layout.part_bounds(WorkbenchPart::Titlebar),
        Some(Rect::from_xywh(0.0, 0.0, 1_000.0, 32.0))
    );
    assert_eq!(
        layout.part_bounds(WorkbenchPart::Sessions),
        Some(Rect::from_xywh(0.0, 32.0, 200.0, 668.0))
    );
    assert_eq!(
        layout.part_bounds(WorkbenchPart::Main),
        Some(Rect::from_xywh(200.0, 32.0, 480.0, 668.0))
    );
    assert_eq!(
        layout.part_bounds(WorkbenchPart::Inspector),
        Some(Rect::from_xywh(680.0, 32.0, 320.0, 668.0))
    );
}

#[test]
fn workbench_omits_collapsed_parts_without_changing_main_origin() {
    let layout = spec(SidebarVisibility::Collapsed, SidebarVisibility::Collapsed)
        .for_viewport(LogicalViewport {
            width: 1_000.0,
            height: 700.0,
        })
        .expect("viewport should fit the Workbench");

    assert_eq!(layout.sessions(), None);
    assert_eq!(layout.inspector(), None);
    assert_eq!(layout.main(), Rect::from_xywh(0.0, 32.0, 1_000.0, 668.0));
}

#[test]
fn workbench_rejects_compact_viewports_before_mounting_parts() {
    assert!(
        spec(SidebarVisibility::Expanded, SidebarVisibility::Expanded)
            .for_viewport(LogicalViewport {
                width: 239.0,
                height: 700.0,
            })
            .is_none()
    );
    assert!(
        spec(SidebarVisibility::Expanded, SidebarVisibility::Expanded)
            .for_viewport(LogicalViewport {
                width: 1_000.0,
                height: 179.0,
            })
            .is_none()
    );
}
