/// Returns whether the current process is running under Windows Subsystem for Linux.
pub fn is_wsl() -> bool {
    #[cfg(target_os = "linux")]
    {
        if std::env::var_os("WSL_DISTRO_NAME").is_some() {
            return true;
        }
        std::fs::read_to_string("/proc/version")
            .is_ok_and(|version| version.to_lowercase().contains("microsoft"))
    }
    #[cfg(not(target_os = "linux"))]
    {
        false
    }
}
