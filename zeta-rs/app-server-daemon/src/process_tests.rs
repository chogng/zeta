use std::fs;

use super::resolve_daemon_executable;

#[test]
fn resolves_the_selected_daemon_in_its_package_directory() {
    let root = tempfile::tempdir().unwrap();
    let binary_directory = root.path().join("package").join("bin");
    fs::create_dir_all(&binary_directory).unwrap();
    let daemon = binary_directory.join(if cfg!(windows) {
        "zeta-app-server-daemon.exe"
    } else {
        "zeta-app-server-daemon"
    });
    fs::copy(std::env::current_exe().unwrap(), &daemon).unwrap();

    let resolved = resolve_daemon_executable(&daemon).unwrap();

    assert_eq!(resolved.path, dunce::canonicalize(&daemon).unwrap());
    assert_eq!(resolved.path.parent(), Some(binary_directory.as_path()));
}
