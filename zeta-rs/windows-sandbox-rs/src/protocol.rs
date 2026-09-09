#![forbid(unsafe_code)]

pub(crate) const RUNNER_PROBE: &str = "zeta-windows-command-runner-v1";
pub(crate) const ERROR_PREFIX: &str = "zeta-windows-sandbox:";
pub(crate) const ENFORCEMENT_FAILURE_EXIT_CODE: i32 = 125;
#[cfg(any(target_os = "windows", test))]
pub(crate) const INNER_RESERVED_EXIT_CODE_REMAP: i32 = 124;
pub(crate) const PROBE_FLAG: &str = "--probe";
pub(crate) const ACCESS_FLAG: &str = "--access";
pub(crate) const DIR_FLAG: &str = "--dir";
pub(crate) const CWD_FLAG: &str = "--cwd";
pub(crate) const COMMAND_SEPARATOR: &str = "--";
pub(crate) const READ_ONLY_ACCESS: &str = "read-only";
pub(crate) const DIR_WRITE_ACCESS: &str = "dir-write";

#[cfg(any(target_os = "windows", test))]
pub(crate) fn remap_inner_exit_code(exit_code: i32) -> i32 {
    if exit_code == ENFORCEMENT_FAILURE_EXIT_CODE {
        INNER_RESERVED_EXIT_CODE_REMAP
    } else {
        exit_code
    }
}
