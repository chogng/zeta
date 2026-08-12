use super::{ClientEvent, map_event};
use crossterm::event::{self, Event};
use std::io;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::mpsc::Receiver;
use std::sync::mpsc::RecvTimeoutError;
use std::sync::mpsc::SyncSender;
use std::sync::mpsc::TrySendError;
use std::thread::{self, JoinHandle};
use std::time::Duration;
use zeta_app_server_client::AppServerEvents;

#[cfg(unix)]
use signal_hook::SigId;
#[cfg(unix)]
use signal_hook::consts::{SIGINT, SIGTERM};

const TERMINAL_POLL_INTERVAL: Duration = Duration::from_millis(25);
const EVENT_QUEUE_CAPACITY: usize = 1_024;
const EVENT_SEND_RETRY: Duration = Duration::from_millis(1);

pub(crate) enum RuntimeEvent {
    Terminal(Event),
    Client(ClientEvent),
    Tick,
    TerminationRequested,
    TerminalFailed(io::Error),
}

pub(crate) struct EventPump {
    receiver: Receiver<RuntimeEvent>,
    stop: Arc<AtomicBool>,
    terminal: Option<JoinHandle<()>>,
    client: Option<JoinHandle<()>>,
    #[cfg(unix)]
    _termination_signals: SignalRegistrations,
}

impl EventPump {
    pub(crate) fn start(events: AppServerEvents) -> Result<Self, io::Error> {
        let (sender, receiver) = mpsc::sync_channel(EVENT_QUEUE_CAPACITY);
        let stop = Arc::new(AtomicBool::new(false));
        let termination_requested = Arc::new(AtomicBool::new(false));
        #[cfg(unix)]
        let termination_signals =
            SignalRegistrations::register(Arc::clone(&termination_requested))?;
        let terminal_stop = Arc::clone(&stop);
        let terminal_termination_requested = Arc::clone(&termination_requested);
        let terminal_sender = sender.clone();
        let terminal = thread::Builder::new()
            .name("zeta-tui-terminal-events".into())
            .spawn(move || {
                while !terminal_stop.load(Ordering::Acquire) {
                    if let Some(event) = termination_event(&terminal_termination_requested) {
                        let _ = send_event(
                            &terminal_sender,
                            event,
                            &terminal_stop,
                            EventOverflow::Wait,
                        );
                        return;
                    }
                    match event::poll(TERMINAL_POLL_INTERVAL) {
                        Ok(true) => match event::read() {
                            Ok(event) => {
                                if !send_event(
                                    &terminal_sender,
                                    RuntimeEvent::Terminal(event),
                                    &terminal_stop,
                                    EventOverflow::Wait,
                                ) {
                                    return;
                                }
                            }
                            Err(error) => {
                                let _ = send_event(
                                    &terminal_sender,
                                    RuntimeEvent::TerminalFailed(error),
                                    &terminal_stop,
                                    EventOverflow::Wait,
                                );
                                return;
                            }
                        },
                        Ok(false) => {
                            if !send_event(
                                &terminal_sender,
                                RuntimeEvent::Tick,
                                &terminal_stop,
                                EventOverflow::Drop,
                            ) {
                                return;
                            }
                        }
                        Err(error) => {
                            let _ = send_event(
                                &terminal_sender,
                                RuntimeEvent::TerminalFailed(error),
                                &terminal_stop,
                                EventOverflow::Wait,
                            );
                            return;
                        }
                    }
                }
            })?;

        let client_sender = sender;
        let client_stop = Arc::clone(&stop);
        let client = match thread::Builder::new()
            .name("zeta-tui-client-events".into())
            .spawn(move || {
                while !client_stop.load(Ordering::Acquire) {
                    match events.recv_timeout(TERMINAL_POLL_INTERVAL) {
                        Ok(event) => {
                            if let Some(event) = map_event(event)
                                && !send_event(
                                    &client_sender,
                                    RuntimeEvent::Client(event),
                                    &client_stop,
                                    EventOverflow::Wait,
                                )
                            {
                                return;
                            }
                        }
                        Err(RecvTimeoutError::Timeout) => {}
                        Err(RecvTimeoutError::Disconnected) => return,
                    }
                }
            }) {
            Ok(client) => client,
            Err(error) => {
                stop.store(true, Ordering::Release);
                let _ = terminal.join();
                return Err(error);
            }
        };

        Ok(Self {
            receiver,
            stop,
            terminal: Some(terminal),
            client: Some(client),
            #[cfg(unix)]
            _termination_signals: termination_signals,
        })
    }

    pub(crate) fn recv(&self) -> Result<RuntimeEvent, io::Error> {
        self.receiver
            .recv()
            .map_err(|_| io::Error::new(io::ErrorKind::BrokenPipe, "TUI event sources stopped"))
    }

    pub(crate) fn shutdown(mut self) -> Result<(), io::Error> {
        self.stop.store(true, Ordering::Release);
        join(&mut self.terminal, "terminal event pump")?;
        join(&mut self.client, "App Server event pump")
    }
}

#[derive(Clone, Copy)]
enum EventOverflow {
    Drop,
    Wait,
}

fn send_event(
    sender: &SyncSender<RuntimeEvent>,
    mut event: RuntimeEvent,
    stop: &AtomicBool,
    overflow: EventOverflow,
) -> bool {
    loop {
        match sender.try_send(event) {
            Ok(()) => return true,
            Err(TrySendError::Disconnected(_)) => return false,
            Err(TrySendError::Full(_)) if matches!(overflow, EventOverflow::Drop) => return true,
            Err(TrySendError::Full(returned)) => {
                if stop.load(Ordering::Acquire) {
                    return false;
                }
                event = returned;
                thread::sleep(EVENT_SEND_RETRY);
            }
        }
    }
}

impl Drop for EventPump {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
    }
}

fn join(task: &mut Option<JoinHandle<()>>, name: &str) -> Result<(), io::Error> {
    task.take()
        .map(JoinHandle::join)
        .transpose()
        .map(|_| ())
        .map_err(|_| io::Error::other(format!("{name} panicked")))
}

fn termination_event(requested: &AtomicBool) -> Option<RuntimeEvent> {
    requested
        .swap(false, Ordering::AcqRel)
        .then_some(RuntimeEvent::TerminationRequested)
}

#[cfg(unix)]
struct SignalRegistrations {
    ids: Vec<SigId>,
}

#[cfg(unix)]
impl SignalRegistrations {
    fn register(requested: Arc<AtomicBool>) -> io::Result<Self> {
        let mut registrations = Self { ids: Vec::new() };
        for signal in [SIGINT, SIGTERM] {
            let id = signal_hook::flag::register(signal, Arc::clone(&requested))?;
            registrations.ids.push(id);
        }
        Ok(registrations)
    }
}

#[cfg(unix)]
impl Drop for SignalRegistrations {
    fn drop(&mut self) {
        for id in self.ids.drain(..) {
            signal_hook::low_level::unregister(id);
        }
    }
}

#[cfg(test)]
#[path = "event_pump_tests.rs"]
mod tests;
