use crate::client::ClientEvent;
use crate::client::ClientEventSource;
use crate::host::TerminationSource;
use crate::terminal::TerminalEvent;
use crate::terminal::TerminalEventSource;
use crossterm::event::Event;
use crossterm::event::MouseEventKind;
use std::collections::VecDeque;
use std::io;
use std::sync::Arc;
use std::sync::Condvar;
use std::sync::Mutex;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::time::Duration;
use zeta_app_server_client::AppServerEvents;

const EVENT_QUEUE_CAPACITY: usize = 1_024;

pub(super) enum RuntimeEvent {
    Terminal(TerminalEvent),
    Client(ClientEvent),
    TerminationRequested,
}

pub(super) struct EventPump {
    queue: Arc<RuntimeQueue>,
    stop: Arc<AtomicBool>,
    terminal: TerminalEventSource,
    client: ClientEventSource,
    _termination: TerminationSource,
}

impl EventPump {
    pub(super) fn start(events: AppServerEvents) -> Result<Self, io::Error> {
        let queue = Arc::new(RuntimeQueue::default());
        let stop = Arc::new(AtomicBool::new(false));
        let termination = TerminationSource::register()?;
        let termination_request = termination.request();
        let terminal_stop = Arc::clone(&stop);
        let terminal_queue = Arc::clone(&queue);
        let terminal = TerminalEventSource::start(Arc::clone(&stop), move |event| {
            if termination_request.take() {
                let _ = terminal_queue.request_termination(&terminal_stop);
                return false;
            }
            if matches!(event, TerminalEvent::Tick) {
                terminal_queue.push_tick(&terminal_stop)
            } else {
                terminal_queue.push_priority(RuntimeEvent::Terminal(event), &terminal_stop)
            }
        })?;

        let client_queue = Arc::clone(&queue);
        let client_stop = Arc::clone(&stop);
        let client = match ClientEventSource::start(events, Arc::clone(&stop), move |event| {
            client_queue.push_client(RuntimeEvent::Client(event), &client_stop)
        }) {
            Ok(client) => client,
            Err(error) => {
                stop.store(true, Ordering::Release);
                queue.close();
                let mut terminal = terminal;
                let _ = terminal.join();
                return Err(error);
            }
        };

        Ok(Self {
            queue,
            stop,
            terminal,
            client,
            _termination: termination,
        })
    }

    pub(super) fn recv(&self) -> Result<RuntimeEvent, io::Error> {
        self.queue.recv(None)?.ok_or_else(queue_closed)
    }

    pub(super) fn recv_timeout(
        &self,
        timeout: Duration,
    ) -> Result<Option<RuntimeEvent>, io::Error> {
        self.queue.recv(Some(timeout))
    }

    pub(super) fn shutdown(mut self) -> Result<(), io::Error> {
        self.stop.store(true, Ordering::Release);
        self.queue.close();
        let terminal_result = self.terminal.join();
        let client_result = self.client.join();
        terminal_result.and(client_result)
    }
}

#[derive(Default)]
struct RuntimeQueue {
    state: Mutex<RuntimeQueueState>,
    available: Condvar,
    space: Condvar,
}

#[derive(Default)]
struct RuntimeQueueState {
    priority: VecDeque<RuntimeEvent>,
    client: VecDeque<RuntimeEvent>,
    tick_pending: bool,
    termination_requested: bool,
    closed: bool,
}

impl RuntimeQueue {
    fn push_priority(&self, event: RuntimeEvent, stop: &AtomicBool) -> bool {
        self.push(event, stop, QueueLane::Priority)
    }

    fn push_client(&self, event: RuntimeEvent, stop: &AtomicBool) -> bool {
        self.push(event, stop, QueueLane::Client)
    }

    fn push(&self, event: RuntimeEvent, stop: &AtomicBool, lane: QueueLane) -> bool {
        let mut state = self.state.lock().unwrap();
        let mut event = event;
        loop {
            if matches!(lane, QueueLane::Priority) {
                match coalesce_pointer_input(&mut state.priority, event) {
                    Some(pending) => event = pending,
                    None => {
                        self.available.notify_one();
                        return true;
                    }
                }
            }
            if lane.queue(&mut state).len() < EVENT_QUEUE_CAPACITY
                || state.closed
                || stop.load(Ordering::Acquire)
            {
                break;
            }
            state = self.space.wait(state).unwrap();
        }
        if state.closed || stop.load(Ordering::Acquire) {
            return false;
        }
        lane.queue(&mut state).push_back(event);
        self.available.notify_one();
        true
    }

    fn push_tick(&self, stop: &AtomicBool) -> bool {
        let mut state = self.state.lock().unwrap();
        if state.closed || stop.load(Ordering::Acquire) {
            return false;
        }
        state.tick_pending = true;
        self.available.notify_one();
        true
    }

    fn request_termination(&self, stop: &AtomicBool) -> bool {
        let mut state = self.state.lock().unwrap();
        if state.closed || stop.load(Ordering::Acquire) {
            return false;
        }
        state.termination_requested = true;
        self.available.notify_one();
        true
    }

    fn recv(&self, timeout: Option<Duration>) -> Result<Option<RuntimeEvent>, io::Error> {
        let mut state = self.state.lock().unwrap();
        if let Some(timeout) = timeout {
            let (next, wait) = self
                .available
                .wait_timeout_while(state, timeout, |state| state.is_empty())
                .unwrap();
            state = next;
            if wait.timed_out() && state.is_empty() {
                return Ok(None);
            }
        } else {
            state = self
                .available
                .wait_while(state, |state| state.is_empty())
                .unwrap();
        }

        let event = state.pop();
        self.space.notify_all();
        if event.is_none() && state.closed {
            return Err(queue_closed());
        }
        Ok(event)
    }

    fn close(&self) {
        let mut state = self.state.lock().unwrap();
        state.closed = true;
        self.available.notify_all();
        self.space.notify_all();
    }
}

fn coalesce_pointer_input(
    queue: &mut VecDeque<RuntimeEvent>,
    incoming: RuntimeEvent,
) -> Option<RuntimeEvent> {
    let Some(incoming_kind) = mouse_kind(&incoming) else {
        return Some(incoming);
    };
    let Some(previous_kind) = queue.back().and_then(mouse_kind) else {
        return Some(incoming);
    };

    let replace_previous = matches!(
        (previous_kind, incoming_kind),
        (MouseEventKind::Moved, MouseEventKind::Moved)
    ) || matches!(
        (previous_kind, incoming_kind),
        (MouseEventKind::Drag(previous), MouseEventKind::Drag(incoming)) if previous == incoming
    );
    if replace_previous {
        *queue
            .back_mut()
            .expect("the previous pointer event is still queued") = incoming;
        return None;
    }

    if matches!(
        (previous_kind, incoming_kind),
        (MouseEventKind::Drag(previous), MouseEventKind::Up(incoming)) if previous == incoming
    ) {
        queue.pop_back();
    }
    Some(incoming)
}

fn mouse_kind(event: &RuntimeEvent) -> Option<MouseEventKind> {
    match event {
        RuntimeEvent::Terminal(TerminalEvent::Input(Event::Mouse(mouse))) => Some(mouse.kind),
        RuntimeEvent::Terminal(_)
        | RuntimeEvent::Client(_)
        | RuntimeEvent::TerminationRequested => None,
    }
}

impl RuntimeQueueState {
    fn is_empty(&self) -> bool {
        !self.termination_requested
            && self.priority.is_empty()
            && self.client.is_empty()
            && !self.tick_pending
            && !self.closed
    }

    fn pop(&mut self) -> Option<RuntimeEvent> {
        if std::mem::take(&mut self.termination_requested) {
            return Some(RuntimeEvent::TerminationRequested);
        }
        if let Some(event) = self.priority.pop_front() {
            return Some(event);
        }
        if let Some(event) = self.client.pop_front() {
            return Some(event);
        }
        if std::mem::take(&mut self.tick_pending) {
            return Some(RuntimeEvent::Terminal(TerminalEvent::Tick));
        }
        None
    }
}

#[derive(Clone, Copy)]
enum QueueLane {
    Priority,
    Client,
}

impl QueueLane {
    fn queue<'a>(&self, state: &'a mut RuntimeQueueState) -> &'a mut VecDeque<RuntimeEvent> {
        match self {
            Self::Priority => &mut state.priority,
            Self::Client => &mut state.client,
        }
    }
}

fn queue_closed() -> io::Error {
    io::Error::new(io::ErrorKind::BrokenPipe, "TUI event sources stopped")
}

impl Drop for EventPump {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        self.queue.close();
    }
}

#[cfg(test)]
#[path = "event_pump_tests.rs"]
mod tests;
