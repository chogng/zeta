use std::io;
use std::io::Read;
use std::io::Write;
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use std::time::Instant;

use zeta_uds::UnixListener;
use zeta_uds::UnixStream;

use crate::LocalConnections;
use crate::LocalSocketAccept;
use crate::PollingLocalListener;

#[test]
fn polling_listener_returns_blocking_connections() -> io::Result<()> {
    let socket_directory = tempfile::tempdir()?;
    let socket_path = socket_directory.path().join("polling-listener.sock");
    let listener = PollingLocalListener::new(UnixListener::bind(&socket_path)?)?;
    assert!(matches!(
        listener.poll_accept()?,
        LocalSocketAccept::Pending
    ));

    let mut client = UnixStream::connect(&socket_path)?;
    let mut server = wait_for_connection(&listener)?;
    let sender = thread::spawn(move || -> io::Result<()> {
        thread::sleep(Duration::from_millis(50));
        client.write_all(b"ping")
    });

    let mut request = [0; 4];
    server.read_exact(&mut request)?;
    assert_eq!(&request, b"ping");
    sender.join().expect("socket sender thread panicked")?;
    Ok(())
}

#[test]
fn connection_guard_tracks_and_removes_connection() -> io::Result<()> {
    let (stream, _peer) = UnixStream::pair()?;
    let connections = Arc::new(LocalConnections::new());
    let guard = connections.register(stream.try_clone()?)?;
    assert_eq!(connections.len(), 1);

    drop(guard);
    assert!(connections.is_empty());
    Ok(())
}

fn wait_for_connection(listener: &PollingLocalListener) -> io::Result<UnixStream> {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        match listener.poll_accept()? {
            LocalSocketAccept::Accepted(stream) => return Ok(stream),
            LocalSocketAccept::Pending if Instant::now() < deadline => {
                thread::sleep(Duration::from_millis(10));
            }
            LocalSocketAccept::Pending => {
                return Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    "listener did not accept a connection",
                ));
            }
            LocalSocketAccept::Rejected(error) => return Err(error),
        }
    }
}
