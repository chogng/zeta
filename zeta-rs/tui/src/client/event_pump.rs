use super::{ClientEvent, map_event};
use crossterm::event::{self, Event};
use std::io;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::thread::{self, JoinHandle};
use std::time::Duration;
use zeta_app_server_client::AppServerEvents;

const TERMINAL_POLL_INTERVAL: Duration = Duration::from_millis(25);

pub(crate) enum RuntimeEvent {
    Terminal(Event),
    Client(ClientEvent),
    Tick,
    TerminalFailed(io::Error),
}

pub(crate) struct EventPump {
    receiver: Receiver<RuntimeEvent>,
    stop: Arc<AtomicBool>,
    terminal: Option<JoinHandle<()>>,
    client: Option<JoinHandle<()>>,
}

impl EventPump {
    pub(crate) fn start(events: AppServerEvents) -> Result<Self, io::Error> {
        let (sender, receiver) = mpsc::channel();
        let stop = Arc::new(AtomicBool::new(false));
        let terminal_stop = Arc::clone(&stop);
        let terminal_sender = sender.clone();
        let terminal = thread::Builder::new()
            .name("zeta-tui-terminal-events".into())
            .spawn(move || {
                while !terminal_stop.load(Ordering::Acquire) {
                    match event::poll(TERMINAL_POLL_INTERVAL) {
                        Ok(true) => match event::read() {
                            Ok(event) => {
                                if terminal_sender.send(RuntimeEvent::Terminal(event)).is_err() {
                                    return;
                                }
                            }
                            Err(error) => {
                                let _ = terminal_sender.send(RuntimeEvent::TerminalFailed(error));
                                return;
                            }
                        },
                        Ok(false) => {
                            if terminal_sender.send(RuntimeEvent::Tick).is_err() {
                                return;
                            }
                        }
                        Err(error) => {
                            let _ = terminal_sender.send(RuntimeEvent::TerminalFailed(error));
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
                                && client_sender.send(RuntimeEvent::Client(event)).is_err()
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
