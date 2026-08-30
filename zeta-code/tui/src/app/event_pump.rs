use crate::client::ClientEvent;
use crate::client::ClientEventSource;
use crate::host::TerminationSource;
use crate::terminal::TerminalEvent;
use crate::terminal::TerminalEventSource;
use std::io;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::sync::mpsc;
use std::sync::mpsc::Receiver;
use std::sync::mpsc::SyncSender;
use std::sync::mpsc::TrySendError;
use std::thread;
use std::time::Duration;
use zeta_app_server_client::AppServerEvents;

const EVENT_QUEUE_CAPACITY: usize = 1_024;
const EVENT_SEND_RETRY: Duration = Duration::from_millis(1);

pub(super) enum RuntimeEvent {
    Terminal(TerminalEvent),
    Client(ClientEvent),
    TerminationRequested,
}

pub(super) struct EventPump {
    receiver: Receiver<RuntimeEvent>,
    stop: Arc<AtomicBool>,
    terminal: TerminalEventSource,
    client: ClientEventSource,
    _termination: TerminationSource,
}

impl EventPump {
    pub(super) fn start(events: AppServerEvents) -> Result<Self, io::Error> {
        let (sender, receiver) = mpsc::sync_channel(EVENT_QUEUE_CAPACITY);
        let stop = Arc::new(AtomicBool::new(false));
        let termination = TerminationSource::register()?;
        let termination_request = termination.request();
        let terminal_stop = Arc::clone(&stop);
        let terminal_sender = sender.clone();
        let terminal = TerminalEventSource::start(Arc::clone(&stop), move |event| {
            if termination_request.take() {
                let _ = send_event(
                    &terminal_sender,
                    RuntimeEvent::TerminationRequested,
                    &terminal_stop,
                    EventOverflow::Wait,
                );
                return false;
            }
            let overflow = if matches!(event, TerminalEvent::Tick) {
                EventOverflow::Drop
            } else {
                EventOverflow::Wait
            };
            send_event(
                &terminal_sender,
                RuntimeEvent::Terminal(event),
                &terminal_stop,
                overflow,
            )
        })?;

        let client_sender = sender;
        let client_stop = Arc::clone(&stop);
        let client = match ClientEventSource::start(events, Arc::clone(&stop), move |event| {
            send_event(
                &client_sender,
                RuntimeEvent::Client(event),
                &client_stop,
                EventOverflow::Wait,
            )
        }) {
            Ok(client) => client,
            Err(error) => {
                stop.store(true, Ordering::Release);
                let mut terminal = terminal;
                let _ = terminal.join();
                return Err(error);
            }
        };

        Ok(Self {
            receiver,
            stop,
            terminal,
            client,
            _termination: termination,
        })
    }

    pub(super) fn recv(&self) -> Result<RuntimeEvent, io::Error> {
        self.receiver
            .recv()
            .map_err(|_| io::Error::new(io::ErrorKind::BrokenPipe, "TUI event sources stopped"))
    }

    pub(super) fn shutdown(mut self) -> Result<(), io::Error> {
        self.stop.store(true, Ordering::Release);
        let terminal_result = self.terminal.join();
        let client_result = self.client.join();
        terminal_result.and(client_result)
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
