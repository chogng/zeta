use std::fs;

use tempfile::tempdir;

use super::RemoteServerOptions;
use super::unix::endpoint_identity;

#[test]
fn endpoint_identity_changes_when_the_runtime_is_rebuilt_at_the_same_path() {
    let root = tempdir().unwrap();
    let profile = root.path().join("profile");
    let dir = root.path().join("dir");
    let runtime = root.path().join("zeta");
    fs::create_dir(&profile).unwrap();
    fs::create_dir(&dir).unwrap();
    fs::write(&runtime, b"first development runtime").unwrap();
    let options = RemoteServerOptions::new(&profile, &dir);

    let first = endpoint_identity(&options, &profile, &dir, &runtime).unwrap();
    let unchanged = endpoint_identity(&options, &profile, &dir, &runtime).unwrap();
    fs::write(&runtime, b"second development runtime generation").unwrap();
    let rebuilt = endpoint_identity(&options, &profile, &dir, &runtime).unwrap();

    assert_eq!(first, unchanged);
    assert_ne!(first, rebuilt);
}
