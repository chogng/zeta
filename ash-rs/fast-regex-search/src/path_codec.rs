use std::path::Path;
use std::path::PathBuf;

#[cfg(unix)]
pub(crate) fn encode(path: &Path) -> Vec<u8> {
    use std::os::unix::ffi::OsStrExt;

    path.as_os_str().as_bytes().to_vec()
}

#[cfg(unix)]
pub(crate) fn decode(bytes: &[u8]) -> Result<PathBuf, ()> {
    use std::ffi::OsString;
    use std::os::unix::ffi::OsStringExt;

    Ok(PathBuf::from(OsString::from_vec(bytes.to_vec())))
}

#[cfg(windows)]
pub(crate) fn encode(path: &Path) -> Vec<u8> {
    use std::os::windows::ffi::OsStrExt;

    path.as_os_str()
        .encode_wide()
        .flat_map(u16::to_le_bytes)
        .collect()
}

#[cfg(windows)]
pub(crate) fn decode(bytes: &[u8]) -> Result<PathBuf, ()> {
    use std::ffi::OsString;
    use std::os::windows::ffi::OsStringExt;

    let mut chunks = bytes.chunks_exact(2);
    let units = chunks
        .by_ref()
        .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]))
        .collect::<Vec<_>>();
    if !chunks.remainder().is_empty() {
        return Err(());
    }
    Ok(PathBuf::from(OsString::from_wide(&units)))
}

#[cfg(not(any(unix, windows)))]
pub(crate) fn encode(path: &Path) -> Vec<u8> {
    path.to_string_lossy().into_owned().into_bytes()
}

#[cfg(not(any(unix, windows)))]
pub(crate) fn decode(bytes: &[u8]) -> Result<PathBuf, ()> {
    String::from_utf8(bytes.to_vec())
        .map(PathBuf::from)
        .map_err(|_| ())
}

#[cfg(test)]
#[path = "path_codec_tests.rs"]
mod tests;
