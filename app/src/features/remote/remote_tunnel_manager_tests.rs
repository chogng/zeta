use std::num::NonZeroU16;

use zeta_ui::TextInputCommand;

use super::RemoteTunnelEvent;
use super::RemoteTunnelId;
use super::RemoteTunnelLifecycle;
use super::RemoteTunnelManagerState;
use super::RemoteTunnelUpdate;
use super::remote_tunnel_stop_id;

#[test]
fn manager_validates_ports_and_prevents_duplicate_forwards() {
    let mut state = RemoteTunnelManagerState::default();
    state.open("build.example", None);
    state.apply_remote_port(TextInputCommand::Insert("0".into()));
    assert!(state.start_request().is_none());
    assert!(state.status().unwrap().1);

    state.apply_remote_port(TextInputCommand::SelectAll);
    state.apply_remote_port(TextInputCommand::Insert("3000".into()));
    let port = state.start_request().unwrap();
    state.start_succeeded(RemoteTunnelId::new(7), port);
    state.apply_remote_port(TextInputCommand::Insert("3000".into()));
    assert!(state.start_request().is_none());
    assert!(state.status().unwrap().0.contains("already has a tunnel"));
}

#[test]
fn manager_projects_ready_stopping_and_terminal_events() {
    let mut state = RemoteTunnelManagerState::default();
    state.open("build.example", None);
    state.start_succeeded(RemoteTunnelId::new(9), NonZeroU16::new(3_000).unwrap());
    assert_eq!(
        state.tunnels()[0].lifecycle(),
        RemoteTunnelLifecycle::Starting
    );

    let ready = event(
        9,
        RemoteTunnelUpdate::Ready {
            local_port: NonZeroU16::new(49_152).unwrap(),
        },
    );
    assert!(state.handle_event(&ready));
    assert_eq!(state.tunnels()[0].lifecycle(), RemoteTunnelLifecycle::Ready);
    assert_eq!(state.tunnels()[0].local_port().unwrap().get(), 49_152);
    assert!(state.handle_event(&event(9, RemoteTunnelUpdate::Recovering { attempt: 1 })));
    assert_eq!(
        state.tunnels()[0].lifecycle(),
        RemoteTunnelLifecycle::Recovering
    );
    assert!(state.status().unwrap().0.contains("attempt 1"));
    assert!(state.handle_event(&ready));
    assert_eq!(state.tunnels()[0].lifecycle(), RemoteTunnelLifecycle::Ready);
    assert!(state.stop_request(RemoteTunnelId::new(9)));
    assert_eq!(
        state.tunnels()[0].lifecycle(),
        RemoteTunnelLifecycle::Stopping
    );
    assert!(state.handle_event(&event(9, RemoteTunnelUpdate::Stopped)));
    assert!(state.tunnels().is_empty());
}

#[test]
fn dismissing_the_manager_preserves_native_tunnel_records() {
    let mut state = RemoteTunnelManagerState::default();
    state.open("build.example", None);
    state.start_succeeded(RemoteTunnelId::new(11), NonZeroU16::new(8_080).unwrap());

    state.dismiss();

    assert!(!state.is_open());
    assert_eq!(state.tunnels().len(), 1);
}

#[test]
fn tunnel_controls_keep_semantic_identity_when_an_earlier_row_exits() {
    let mut state = RemoteTunnelManagerState::default();
    state.open("build.example", None);
    state.start_succeeded(RemoteTunnelId::new(21), NonZeroU16::new(3_000).unwrap());
    state.start_succeeded(RemoteTunnelId::new(22), NonZeroU16::new(4_000).unwrap());
    let second_stop = remote_tunnel_stop_id(RemoteTunnelId::new(22));

    assert!(state.handle_event(&event(21, RemoteTunnelUpdate::Stopped)));

    assert_eq!(state.stop_id(second_stop), Some(RemoteTunnelId::new(22)));
}

fn event(tunnel_id: u32, update: RemoteTunnelUpdate) -> RemoteTunnelEvent {
    RemoteTunnelEvent::new(
        RemoteTunnelId::new(tunnel_id),
        NonZeroU16::new(3_000).unwrap(),
        update,
    )
}
