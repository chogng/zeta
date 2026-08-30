use super::RuntimeEvent;
use super::RuntimeQueue;
use crate::client::ClientEvent;
use crate::terminal::TerminalEvent;
use crossterm::event::Event;
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
