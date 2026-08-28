use super::PaneKey;
use super::TabContextMenuOutcome;
use super::WorkbenchHost;
use crate::LogicalViewport;
use crate::PaneInput;
use crate::PaneSplitDirection;
use crate::TabInput;
use crate::TabInputKey;
use crate::TabInputMetadata;
use crate::TabStatus;
use zui::ui::Point;
use zui::ui::TextInputCommand;

fn session_id(value: &str) -> zeta_protocol::SessionId {
    zeta_protocol::SessionId::new(value).expect("valid session id")
}

fn session_input(id: zeta_protocol::SessionId) -> TabInput {
    TabInput::session(
        id,
        TabInputMetadata::new("Session", "/workspace").with_status(TabStatus::busy("Active")),
    )
}

#[test]
fn host_mounts_binding_by_tab_pane_and_input_identity() {
    let session = session_id("session-1");
    let tab = TabInputKey::session(session.clone());
    let mut host = WorkbenchHost::new();
    host.upsert_session_input_with(
        session_input(session.clone()),
        PaneInput::terminal(session),
        || "terminal",
    );
    let pane = host
        .workbench()
        .pane_part(&tab)
        .expect("session pane part")
        .root_group();

    let mount = host.mount(&tab, pane).expect("bound input should mount");

    assert_eq!(mount.key().tab(), &tab);
    assert_eq!(mount.pane_id(), pane);
    assert_eq!(*mount.binding(), "terminal");
}

#[test]
fn switching_group_inputs_preserves_each_binding() {
    let session = session_id("session-1");
    let tab = TabInputKey::session(session.clone());
    let mut host = WorkbenchHost::new();
    host.upsert_session_input_with(
        session_input(session.clone()),
        PaneInput::terminal(session.clone()),
        || "terminal",
    );
    let pane = host
        .workbench()
        .pane_part(&tab)
        .expect("session pane part")
        .root_group();
    let terminal = host
        .mount(&tab, pane)
        .expect("terminal mount")
        .key()
        .clone();

    let opened = host
        .open_or_activate_input_with(
            &tab,
            pane,
            PaneInput::files("/workspace".into()),
            || "files",
        )
        .expect("files activation");
    assert!(opened.opened());
    assert_eq!(host.binding(&terminal), Some(&"terminal"));
    assert_eq!(host.binding(opened.current()), Some(&"files"));

    let activated = host
        .open_or_activate_input_with(&tab, pane, PaneInput::terminal(session), || {
            panic!("existing input must not create a replacement binding")
        })
        .expect("terminal activation");
    assert!(!activated.opened());
    assert_eq!(activated.current(), &terminal);
    assert_eq!(host.binding(&terminal), Some(&"terminal"));
}

#[test]
fn closing_a_pane_detaches_all_group_input_bindings() {
    let session = session_id("session-1");
    let tab = TabInputKey::session(session.clone());
    let mut host = WorkbenchHost::new();
    host.upsert_session_input_with(
        session_input(session.clone()),
        PaneInput::terminal(session.clone()),
        || "root",
    );
    let split = host
        .try_split_active_with(
            PaneInput::terminal(session),
            PaneSplitDirection::Horizontal,
            || Ok::<_, std::convert::Infallible>("split-terminal"),
        )
        .expect("binding creation")
        .expect("split input");
    host.open_or_activate_input_with(
        &tab,
        split.pane(),
        PaneInput::files("/workspace".into()),
        || "split-files",
    )
    .expect("second split input");

    let closed = host.close_active_pane().expect("active split pane");
    let active = closed.active_pane();
    let mut bindings = closed.into_bindings();
    bindings.sort_unstable();

    assert_eq!(bindings, vec!["split-files", "split-terminal"]);
    assert_ne!(active, split.pane());
}

#[test]
fn closing_a_tab_detaches_only_its_bindings() {
    let first = session_id("session-1");
    let second = session_id("session-2");
    let first_tab = TabInputKey::session(first.clone());
    let second_tab = TabInputKey::session(second.clone());
    let mut host = WorkbenchHost::new();
    host.upsert_session_input_with(
        session_input(first.clone()),
        PaneInput::terminal(first),
        || "first",
    );
    host.upsert_session_input_with(
        session_input(second.clone()),
        PaneInput::terminal(second),
        || "second",
    );
    let second_key = host.active_mount().expect("second mount").key().clone();

    let (_, bindings) = host.close_tab(&first_tab).expect("first tab should close");

    assert_eq!(bindings, vec!["first"]);
    assert_eq!(host.binding(&second_key), Some(&"second"));
    assert_eq!(
        host.workbench().tab_part().active_tab_key(),
        Some(&second_tab)
    );
}

#[test]
fn host_delegates_layout_without_mutating_model() {
    let host = WorkbenchHost::<()>::new();
    let before = host.workbench().clone();
    let layout = host.layout(
        crate::WorkbenchLayoutSpec::new(
            32.0,
            crate::TabContainerLayoutSpec::new(
                crate::PartVisibility::Collapsed,
                200.0,
                160.0,
                480.0,
                240.0,
            ),
            crate::InspectorLayoutSpec::new(
                crate::PartVisibility::Collapsed,
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
fn pane_key_keeps_input_identity_distinct_inside_one_group() {
    let tab = TabInputKey::Settings;
    let first = PaneKey::new(
        tab.clone(),
        crate::PaneGroupId::ROOT,
        crate::PaneInputId::from_value(1),
    );
    let second = PaneKey::new(
        tab,
        crate::PaneGroupId::ROOT,
        crate::PaneInputId::from_value(2),
    );

    assert_ne!(first, second);
}

#[test]
fn tab_menu_routes_group_selection_and_rename_through_the_workbench_host() {
    let first = session_id("session-1");
    let second = session_id("session-2");
    let first_tab = TabInputKey::session(first.clone());
    let second_tab = TabInputKey::session(second.clone());
    let mut host = WorkbenchHost::new();
    host.upsert_session_input_with(
        session_input(first.clone()),
        PaneInput::terminal(first),
        || (),
    );
    host.upsert_session_input_with(
        session_input(second.clone()),
        PaneInput::terminal(second),
        || (),
    );
    let group = host
        .move_tab_to_new_group(&second_tab, "Review")
        .expect("second tab group");

    assert!(host.open_tab_context_menu(first_tab.clone(), Point::new(20.0, 30.0), None));
    assert_eq!(
        host.activate_tab_context_menu(crate::TabContextMenuAction::MoveToGroup.element_id()),
        TabContextMenuOutcome::Focus(crate::tab_group_menu_element_id(group))
    );
    assert_eq!(
        host.activate_tab_context_menu(crate::tab_group_menu_element_id(group)),
        TabContextMenuOutcome::Changed
    );
    assert_eq!(
        host.workbench().tab_part().input_group(&first_tab),
        Some(group)
    );

    assert!(host.open_tab_context_menu(first_tab.clone(), Point::new(20.0, 30.0), None));
    assert_eq!(
        host.activate_tab_context_menu(crate::TabContextMenuAction::Rename.element_id()),
        TabContextMenuOutcome::Focus(crate::TAB_RENAME_INPUT)
    );
    assert!(host.apply_tab_rename(TextInputCommand::Insert("Build fixes".to_owned())));
    assert!(host.commit_tab_rename());
    let input = host.workbench().tab_part().input(&first_tab).unwrap();
    assert_eq!(host.workbench().tab_part().tab_name(input), "Build fixes");
}
