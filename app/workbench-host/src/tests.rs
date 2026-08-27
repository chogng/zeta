use super::PaneHost;
use super::PaneHostScope;
use super::PaneKey;
use super::WorkbenchHost;
use zeta_workbench::PaneGroupId;
use zeta_workbench::PaneInput;
use zeta_workbench::PanePart;
use zeta_workbench::PaneSplitDirection;
use zeta_workbench::TabInputKey;
use zeta_workbench_layout::LogicalViewport;

#[test]
fn pane_host_mounts_a_product_neutral_binding() {
    let tab_key = TabInputKey::Settings;
    let pane_part = PanePart::with_input(PaneInput::settings());
    let key: PaneKey = (PaneHostScope::Tab(tab_key.clone()), pane_part.root_group());
    let mut host = PaneHost::new();

    let binding_id = host.insert(key.clone(), "settings");
    let mount = host
        .mount(
            &PaneHostScope::Tab(tab_key),
            &pane_part,
            pane_part.root_group(),
        )
        .expect("bound pane should mount");

    assert_eq!(mount.kind(), zeta_workbench::PaneInputKind::Settings);
    assert_eq!(mount.binding_id(), binding_id);
    assert_eq!(*mount.binding(), "settings");
    assert_eq!(mount.input(), &PaneInput::settings());
}

#[test]
fn pane_host_rejects_unbound_and_stale_panes() {
    let scope = PaneHostScope::Tab(TabInputKey::Settings);
    let mut pane_part = PanePart::with_input(PaneInput::settings());
    let root = pane_part.root_group();
    let mut host = PaneHost::<()>::new();

    assert!(host.mount(&scope, &pane_part, root).is_none());

    let stale_pane = pane_part.split_active(PaneSplitDirection::Horizontal);
    let stale_key = (scope.clone(), stale_pane);
    host.bind(stale_key, ());
    assert!(pane_part.close_group(stale_pane).is_some());

    assert!(host.mount(&scope, &pane_part, stale_pane).is_none());
}

#[test]
fn removing_a_tab_only_releases_its_bindings() {
    let closed = TabInputKey::Settings;
    let kept =
        TabInputKey::session(zeta_protocol::SessionId::new("session-1").expect("valid session id"));
    let mut host = PaneHost::new();
    let closed_key: PaneKey = (PaneHostScope::Tab(closed.clone()), PaneGroupId::ROOT);
    let kept_key: PaneKey = (PaneHostScope::Tab(kept.clone()), PaneGroupId::ROOT);
    host.insert(closed_key.clone(), "closed");
    host.insert(kept_key.clone(), "kept");

    assert_eq!(host.remove_tab(&closed), vec!["closed"]);
    assert!(host.binding(&closed_key).is_none());
    assert_eq!(host.binding(&kept_key), Some(&"kept"));
}

#[test]
fn workbench_host_delegates_layout_without_mutating_model() {
    let host = WorkbenchHost::<()>::new();
    let before = host.workbench().clone();
    let layout = host.layout(
        zeta_workbench_layout::WorkbenchLayoutSpec::new(
            32.0,
            zeta_workbench_layout::TabContainerLayoutSpec::new(
                zeta_workbench_layout::PartVisibility::Collapsed,
                200.0,
                160.0,
                480.0,
                240.0,
            ),
            zeta_workbench_layout::InspectorLayoutSpec::new(
                zeta_workbench_layout::PartVisibility::Collapsed,
                320.0,
                240.0,
                560.0,
                240.0,
            ),
        ),
        LogicalViewport {
            width: 1_000.0,
            height: 700.0,
        },
    );

    assert!(layout.is_some());
    assert_eq!(host.workbench(), &before);
}

#[test]
fn closing_a_workbench_tab_cleans_up_its_bindings() {
    let mut host = WorkbenchHost::<&'static str>::new();
    let session_id = zeta_protocol::SessionId::new("session-1").expect("valid session id");
    host.workbench_mut().upsert_session(
        &zeta_protocol::Session {
            session_id: session_id.clone(),
            title: "Session".to_owned(),
            status: zeta_protocol::SessionStatus::Active,
            model: None,
            workspace: None,
            sequence: 1,
            threads: Vec::new(),
        },
        "/workspace",
    );
    let tab_key = zeta_workbench::TabInputKey::session(session_id);
    let pane = host
        .workbench()
        .pane_container(&tab_key)
        .expect("session pane container should exist")
        .pane_part()
        .root_group();
    let key = (PaneHostScope::Tab(tab_key.clone()), pane);
    host.pane_host_mut().insert(key.clone(), "runtime");

    let (closed, bindings) = host.close_tab(&tab_key).expect("session tab should close");

    assert_eq!(
        closed.active_tab(),
        Some(&zeta_workbench::TabInputKey::Settings)
    );
    assert_eq!(bindings, vec!["runtime"]);
    assert!(host.pane_host().binding(&key).is_none());
}
