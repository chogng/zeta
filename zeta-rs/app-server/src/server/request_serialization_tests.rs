use std::sync::Arc;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;
use std::time::Instant;

use super::RequestScheduler;
use zeta_app_server_protocol::protocol::registry::ClientRequestSerializationScope;
use zeta_app_server_protocol::protocol::registry::SerializationAccess;

fn session(id: &str, access: SerializationAccess) -> ClientRequestSerializationScope {
    ClientRequestSerializationScope::Session {
        session_id: id.into(),
        access,
    }
}

fn connection_resource(id: &str) -> ClientRequestSerializationScope {
    ClientRequestSerializationScope::ConnectionResource {
        namespace: "resourceId",
        resource_id: id.into(),
        access: SerializationAccess::Exclusive,
    }
}

fn wait_until_waiting(scheduler: &RequestScheduler, count: usize) {
    let deadline = Instant::now() + Duration::from_secs(1);
    while scheduler.waiting_count() != count {
        assert!(
            Instant::now() < deadline,
            "request did not enter scheduler queue"
        );
        thread::yield_now();
    }
}

#[test]
fn exclusive_requests_with_the_same_key_run_fifo() {
    let scheduler = RequestScheduler::default();
    let first = scheduler
        .acquire(1, session("same", SerializationAccess::Exclusive))
        .unwrap();
    let (sender, receiver) = mpsc::channel();

    let second_scheduler = scheduler.clone();
    let second_sender = sender.clone();
    let second = thread::spawn(move || {
        let permit = second_scheduler
            .acquire(2, session("same", SerializationAccess::Exclusive))
            .unwrap();
        second_sender.send(2).unwrap();
        drop(permit);
    });
    wait_until_waiting(&scheduler, 1);

    let third_scheduler = scheduler.clone();
    let third = thread::spawn(move || {
        let permit = third_scheduler
            .acquire(3, session("same", SerializationAccess::Exclusive))
            .unwrap();
        sender.send(3).unwrap();
        drop(permit);
    });
    wait_until_waiting(&scheduler, 2);

    drop(first);
    assert_eq!(receiver.recv_timeout(Duration::from_secs(1)).unwrap(), 2);
    assert_eq!(receiver.recv_timeout(Duration::from_secs(1)).unwrap(), 3);
    second.join().unwrap();
    third.join().unwrap();
}

#[test]
fn unrelated_session_keys_do_not_block_each_other() {
    let scheduler = RequestScheduler::default();
    let _first = scheduler
        .acquire(1, session("first", SerializationAccess::Exclusive))
        .unwrap();

    let second = scheduler
        .acquire(2, session("second", SerializationAccess::Exclusive))
        .unwrap();
    drop(second);
}

#[test]
fn equal_resource_ids_on_different_connections_are_isolated() {
    let scheduler = RequestScheduler::default();
    let _first = scheduler.acquire(1, connection_resource("same")).unwrap();

    let second = scheduler.acquire(2, connection_resource("same")).unwrap();
    drop(second);
}

#[test]
fn adjacent_shared_reads_run_together_before_a_waiting_writer() {
    let scheduler = Arc::new(RequestScheduler::default());
    let first = scheduler
        .acquire(1, session("same", SerializationAccess::SharedRead))
        .unwrap();
    let second = scheduler
        .acquire(2, session("same", SerializationAccess::SharedRead))
        .unwrap();
    let (sender, receiver) = mpsc::channel();
    let writer_scheduler = Arc::clone(&scheduler);
    let writer = thread::spawn(move || {
        let permit = writer_scheduler
            .acquire(3, session("same", SerializationAccess::Exclusive))
            .unwrap();
        sender.send(()).unwrap();
        drop(permit);
    });
    wait_until_waiting(&scheduler, 1);

    drop(first);
    assert!(receiver.recv_timeout(Duration::from_millis(25)).is_err());
    drop(second);
    receiver.recv_timeout(Duration::from_secs(1)).unwrap();
    writer.join().unwrap();
}

#[test]
fn closing_a_connection_cancels_its_queued_requests() {
    let scheduler = RequestScheduler::default();
    let first = scheduler
        .acquire(1, session("same", SerializationAccess::Exclusive))
        .unwrap();
    let queued_scheduler = scheduler.clone();
    let queued = thread::spawn(move || {
        queued_scheduler.acquire(2, session("same", SerializationAccess::Exclusive))
    });
    wait_until_waiting(&scheduler, 1);

    scheduler.cancel_connection(2);
    assert!(queued.join().unwrap().is_err());
    scheduler.finish_connection(2);
    drop(first);
}
