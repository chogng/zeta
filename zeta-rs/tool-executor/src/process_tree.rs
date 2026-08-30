use std::process::Command;

#[cfg(unix)]
pub(crate) fn isolate(command: &mut Command) {
    use std::os::unix::process::CommandExt;

    command.process_group(0);
}

#[cfg(not(unix))]
pub(crate) fn isolate(_: &mut Command) {}

pub(crate) fn kill(process_group_id: u32) -> std::io::Result<()> {
    zeta_utils_pty::process_group::kill_process_group(process_group_id)
}
