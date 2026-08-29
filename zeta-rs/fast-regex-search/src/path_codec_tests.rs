use super::decode;
use super::encode;
use std::path::PathBuf;

#[cfg(unix)]
#[test]
fn round_trips_non_utf8_paths_without_loss() {
    use std::ffi::OsString;
    use std::os::unix::ffi::OsStringExt;

    let path = PathBuf::from(OsString::from_vec(b"source-\xff.rs".to_vec()));

    assert_eq!(decode(&encode(&path)), Ok(path));
}

#[test]
fn round_trips_nested_paths() {
    let path = PathBuf::from("src/nested/source.rs");

    assert_eq!(decode(&encode(&path)), Ok(path));
}
