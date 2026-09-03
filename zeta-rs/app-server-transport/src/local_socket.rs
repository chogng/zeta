use std::collections::BTreeMap;
use std::io;
use std::net::Shutdown;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;

use zeta_uds::UnixListener;
use zeta_uds::UnixStream;

/// One result from polling a local App Server listener.
pub enum LocalSocketAccept {
    /// No connection is currently ready.
    Pending,
    /// A connection was accepted and normalized for blocking request processing.
    Accepted(UnixStream),
    /// A connection was accepted but could not be normalized for request processing.
    Rejected(io::Error),
}

/// A non-blocking local listener that returns blocking accepted connections.
///
/// The listener is polled by a lifecycle owner, while every accepted stream is
/// normalized before a synchronous protocol reader receives it. This distinction
/// is required on platforms where accepted streams inherit the listener mode.
pub struct PollingLocalListener {
    listener: UnixListener,
}

impl PollingLocalListener {
    /// Configures an existing listener for polling.
    pub fn new(listener: UnixListener) -> io::Result<Self> {
        listener.set_nonblocking(true)?;
        Ok(Self { listener })
    }

    /// Polls for one normalized connection.
    pub fn poll_accept(&self) -> io::Result<LocalSocketAccept> {
        match self.listener.accept() {
            Ok((stream, _address)) => match stream.set_nonblocking(false) {
                Ok(()) => Ok(LocalSocketAccept::Accepted(stream)),
                Err(error) => Ok(LocalSocketAccept::Rejected(error)),
            },
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                Ok(LocalSocketAccept::Pending)
            }
            Err(error) => Err(error),
        }
    }
}

/// Tracks the local connections owned by one App Server process generation.
pub struct LocalConnections {
    next_id: AtomicU64,
    count: AtomicUsize,
    streams: Mutex<BTreeMap<u64, UnixStream>>,
}

impl LocalConnections {
    /// Creates an empty connection set.
    pub fn new() -> Self {
        Self {
            next_id: AtomicU64::new(1),
            count: AtomicUsize::new(0),
            streams: Mutex::new(BTreeMap::new()),
        }
    }

    /// Returns the number of connections whose guards are still alive.
    pub fn len(&self) -> usize {
        self.count.load(Ordering::Acquire)
    }

    /// Returns whether no connection guards are alive.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Registers a connection shutdown handle and returns its lifetime guard.
    pub fn register(
        self: &Arc<Self>,
        shutdown_stream: UnixStream,
    ) -> io::Result<LocalConnectionGuard> {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        self.streams
            .lock()
            .map_err(|_| io::Error::other("local connection lock poisoned"))?
            .insert(id, shutdown_stream);
        self.count.fetch_add(1, Ordering::AcqRel);
        Ok(LocalConnectionGuard {
            connections: Arc::clone(self),
            id,
        })
    }

    /// Shuts down every currently registered connection.
    pub fn shutdown_all(&self) {
        if let Ok(streams) = self.streams.lock() {
            for stream in streams.values() {
                let _ = stream.shutdown(Shutdown::Both);
            }
        }
    }
}

impl Default for LocalConnections {
    fn default() -> Self {
        Self::new()
    }
}

/// Removes one registered connection when its processing task exits.
pub struct LocalConnectionGuard {
    connections: Arc<LocalConnections>,
    id: u64,
}

impl Drop for LocalConnectionGuard {
    fn drop(&mut self) {
        if let Ok(mut streams) = self.connections.streams.lock() {
            streams.remove(&self.id);
        }
        self.connections.count.fetch_sub(1, Ordering::AcqRel);
    }
}
