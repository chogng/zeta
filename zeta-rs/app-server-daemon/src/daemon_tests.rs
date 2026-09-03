use std::io;

use super::is_peer_disconnect;

#[test]
fn peer_disconnects_are_normal_transport_shutdowns() {
    for kind in [
        io::ErrorKind::BrokenPipe,
        io::ErrorKind::ConnectionAborted,
        io::ErrorKind::ConnectionReset,
    ] {
        assert!(is_peer_disconnect(&io::Error::from(kind)));
    }
    assert!(!is_peer_disconnect(&io::Error::from(
        io::ErrorKind::PermissionDenied
    )));
}
