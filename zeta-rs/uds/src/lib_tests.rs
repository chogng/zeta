use std::io;
use std::io::Read;
use std::io::Write;
use std::thread;

use crate::UnixListener;
use crate::UnixStream;

#[test]
fn exchanges_bytes_over_socket_path() -> io::Result<()> {
    let socket_directory = tempfile::tempdir()?;
    let socket_path = socket_directory.path().join("round-trip.sock");
    let listener = UnixListener::bind(&socket_path)?;

    let server = thread::spawn(move || -> io::Result<()> {
        let (mut stream, _) = listener.accept()?;
        let mut request = [0; 4];
        stream.read_exact(&mut request)?;
        assert_eq!(&request, b"ping");
        stream.write_all(b"pong")?;
        Ok(())
    });

    let mut client = UnixStream::connect(&socket_path)?;
    client.write_all(b"ping")?;
    let mut response = [0; 4];
    client.read_exact(&mut response)?;
    assert_eq!(&response, b"pong");

    server.join().expect("socket server thread panicked")?;
    Ok(())
}
