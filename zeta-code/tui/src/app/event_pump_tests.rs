use super::RuntimeEvent;
use super::RuntimeQueue;
use crate::client::ClientEvent;
use crate::terminal::TerminalEvent;
use crossterm::event::Event;
use crossterm::event::KeyModifiers;
use crossterm::event::MouseButton;
use crossterm::event::MouseEvent;
use crossterm::event::MouseEventKind;
use std::sync::atomic::AtomicBool;
use std::time::Duration;

#[test]
fn terminal_input_is_received_before_queued_client_work() {
    let queue = RuntimeQueue::default();
    let stop = AtomicBool::new(false);
    assert!(queue.push_client(RuntimeEvent::Client(ClientEvent::ConnectorsChanged), &stop,));
    assert!(queue.push_priority(
        RuntimeEvent::Terminal(TerminalEvent::Input(Event::FocusGained)),
        &stop,
    ));

    assert!(matches!(
        queue.recv(None).unwrap(),
        Some(RuntimeEvent::Terminal(TerminalEvent::Input(
            Event::FocusGained
        )))
    ));
    assert!(matches!(
        queue.recv(None).unwrap(),
        Some(RuntimeEvent::Client(ClientEvent::ConnectorsChanged))
    ));
}

#[test]
fn termination_is_received_before_every_queue_lane() {
    let queue = RuntimeQueue::default();
    let stop = AtomicBool::new(false);
    assert!(queue.push_client(RuntimeEvent::Client(ClientEvent::ConnectorsChanged), &stop,));
    assert!(queue.push_priority(
        RuntimeEvent::Terminal(TerminalEvent::Input(Event::FocusGained)),
        &stop,
    ));
    assert!(queue.request_termination(&stop));

    assert!(matches!(
        queue.recv(None).unwrap(),
        Some(RuntimeEvent::TerminationRequested)
    ));
}

#[test]
fn ticks_coalesce_while_the_consumer_is_busy() {
    let queue = RuntimeQueue::default();
    let stop = AtomicBool::new(false);
    assert!(queue.push_tick(&stop));
    assert!(queue.push_tick(&stop));
    assert!(queue.push_tick(&stop));

    assert!(matches!(
        queue.recv(None).unwrap(),
        Some(RuntimeEvent::Terminal(TerminalEvent::Tick))
    ));
    assert!(
        queue
            .recv(Some(Duration::from_millis(1)))
            .unwrap()
            .is_none()
    );
}

#[test]
fn consecutive_pointer_movements_keep_only_the_latest_position() {
    let queue = RuntimeQueue::default();
    let stop = AtomicBool::new(false);
    assert!(queue.push_priority(mouse(MouseEventKind::Moved, 2, 3), &stop));
    assert!(queue.push_priority(mouse(MouseEventKind::Moved, 8, 5), &stop));

    assert_mouse(queue.recv(None).unwrap(), MouseEventKind::Moved, 8, 5);
    assert!(
        queue
            .recv(Some(Duration::from_millis(1)))
            .unwrap()
            .is_none()
    );
}

#[test]
fn mouse_up_overtakes_pending_drag_positions() {
    let queue = RuntimeQueue::default();
    let stop = AtomicBool::new(false);
    assert!(queue.push_priority(mouse(MouseEventKind::Down(MouseButton::Left), 1, 1), &stop,));
    assert!(queue.push_priority(mouse(MouseEventKind::Drag(MouseButton::Left), 4, 2), &stop,));
    assert!(queue.push_priority(mouse(MouseEventKind::Drag(MouseButton::Left), 9, 6), &stop,));
    assert!(queue.push_priority(mouse(MouseEventKind::Up(MouseButton::Left), 10, 6), &stop,));

    assert_mouse(
        queue.recv(None).unwrap(),
        MouseEventKind::Down(MouseButton::Left),
        1,
        1,
    );
    assert_mouse(
        queue.recv(None).unwrap(),
        MouseEventKind::Up(MouseButton::Left),
        10,
        6,
    );
    assert!(
        queue
            .recv(Some(Duration::from_millis(1)))
            .unwrap()
            .is_none()
    );
}

fn mouse(kind: MouseEventKind, column: u16, row: u16) -> RuntimeEvent {
    RuntimeEvent::Terminal(TerminalEvent::Input(Event::Mouse(MouseEvent {
        kind,
        column,
        row,
        modifiers: KeyModifiers::NONE,
    })))
}

fn assert_mouse(
    event: Option<RuntimeEvent>,
    expected_kind: MouseEventKind,
    expected_column: u16,
    expected_row: u16,
) {
    let Some(RuntimeEvent::Terminal(TerminalEvent::Input(Event::Mouse(mouse)))) = event else {
        panic!("expected a mouse input event");
    };
    assert_eq!(mouse.kind, expected_kind);
    assert_eq!(mouse.column, expected_column);
    assert_eq!(mouse.row, expected_row);
}
