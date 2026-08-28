use super::*;
use std::ffi::OsString;
use std::path::Path;

#[test]
fn builds_a_typed_bubblewrap_invocation_without_a_shell() {
    let command = BwrapCommandBuilder::new("/usr/bin/bwrap", "/bin/echo")
        .mount("/", "/", MountAccess::ReadOnly)
        .mount("/workspace", "/workspace", MountAccess::ReadWrite)
        .isolate_network()
        .mount_proc()
        .mount_dev()
        .working_directory("/workspace")
        .inner_arguments(["hello", "world"])
        .build();

    assert_eq!(command.program(), Path::new("/usr/bin/bwrap"));
    assert_eq!(
        command.arguments(),
        [
            "--die-with-parent",
            "--new-session",
            "--unshare-user",
            "--unshare-pid",
            "--ro-bind",
            "/",
            "/",
            "--bind",
            "/workspace",
            "/workspace",
            "--unshare-net",
            "--proc",
            "/proc",
            "--dev",
            "/dev",
            "--chdir",
            "/workspace",
            "--",
            "/bin/echo",
            "hello",
            "world",
        ]
        .map(OsString::from)
    );
}
