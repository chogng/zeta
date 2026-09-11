use super::*;
use std::io::Read;
use std::io::Write;

#[test]
fn a_connection_without_the_execution_sid_never_reaches_the_upstream_proxy() {
    let upstream = TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0)).unwrap();
    upstream.set_nonblocking(true).unwrap();
    let reserved = TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0)).unwrap();
    let port = reserved.local_addr().unwrap().port();
    drop(reserved);
    let proxy = Proxy::start(
        port,
        upstream.local_addr().unwrap().port(),
        "S-1-5-21-901-902-903-904".into(),
    )
    .unwrap();
    let mut client = TcpStream::connect((std::net::Ipv4Addr::LOCALHOST, port)).unwrap();
    client
        .set_read_timeout(Some(Duration::from_secs(2)))
        .unwrap();
    client
        .write_all(b"CONNECT example.test:443 HTTP/1.1\r\n\r\n")
        .unwrap();
    let mut byte = [0];
    match client.read(&mut byte) {
        Ok(0) => {}
        Err(error)
            if matches!(
                error.kind(),
                io::ErrorKind::ConnectionReset | io::ErrorKind::ConnectionAborted
            ) => {}
        other => panic!("unauthenticated connection was not closed: {other:?}"),
    }
    assert!(matches!(upstream.accept(), Err(error) if error.kind() == io::ErrorKind::WouldBlock));
    drop(proxy);
}
