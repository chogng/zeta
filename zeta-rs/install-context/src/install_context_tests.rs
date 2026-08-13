use super::*;
use std::fs;
use std::sync::atomic::{AtomicUsize, Ordering};

#[test]
fn package_layout_precedes_legacy_sibling_and_search_path_candidates() {
    let directory = TestDirectory::new();
    let package = directory.path().join("package");
    let binary_directory = package.join("bin");
    let path_directory = package.join("zeta-path");
    let resources_directory = package.join("zeta-resources");
    let search_directory = directory.path().join("system-bin");
    for path in [
        &binary_directory,
        &path_directory,
        &resources_directory,
        &search_directory,
    ] {
        fs::create_dir_all(path).unwrap();
    }
    let metadata_file = package.join(PACKAGE_METADATA_FILE);
    fs::write(&metadata_file, b"{}").unwrap();
    fs::write(resources_directory.join("bwrap"), b"helper").unwrap();
    fs::write(
        resources_directory.join("zeta-command-runner.exe"),
        b"runner",
    )
    .unwrap();
    fs::write(
        resources_directory.join("zeta-windows-sandbox-setup.exe"),
        b"setup",
    )
    .unwrap();
    fs::create_dir(resources_directory.join("skills")).unwrap();
    fs::create_dir(resources_directory.join("product-services")).unwrap();
    fs::write(
        resources_directory.join("product-services/product-services.json"),
        b"{}",
    )
    .unwrap();
    let executable = binary_directory.join("zeta");
    let context = InstallContext::detect(
        Some(&executable),
        None,
        None,
        None,
        None,
        Some(env::join_paths([&search_directory]).unwrap()),
    );

    assert_eq!(context.method(), InstallMethod::Package);
    assert_eq!(
        context.package_layout(),
        Some(&PackageLayout {
            package_directory: package.clone(),
            metadata_file,
            binary_directory: binary_directory.clone(),
            path_directory: path_directory.clone(),
            resources_directory: resources_directory.clone(),
        })
    );
    let expected_candidates =
        expected_ripgrep_candidates([&path_directory, &binary_directory, &search_directory]);
    assert_eq!(
        context.executable_candidates(ManagedExecutable::Ripgrep),
        ExecutableCandidates::SearchPaths(expected_candidates)
    );
    assert_eq!(
        context.bundled_resource("bwrap"),
        Some(resources_directory.join("bwrap"))
    );
    assert_eq!(
        context.bundled_resource_directory("skills"),
        Some(resources_directory.join("skills"))
    );
    assert_eq!(context.bundled_resource_directory("bwrap"), None);
    assert_eq!(context.bundled_resource("skills"), None);
    assert_eq!(
        context.bundled_resource("product-services/product-services.json"),
        Some(resources_directory.join("product-services/product-services.json"))
    );
    let expected_bubblewrap_candidates = [
        resources_directory.join("bwrap"),
        search_directory.join("bwrap"),
    ];
    assert_eq!(
        context.executable_candidates(ManagedExecutable::Bubblewrap),
        ExecutableCandidates::SearchPaths(expected_bubblewrap_candidates.into())
    );
    assert_eq!(
        context.executable_candidates(ManagedExecutable::WindowsCommandRunner),
        ExecutableCandidates::SearchPaths(vec![
            resources_directory.join("zeta-command-runner.exe"),
            search_directory.join("zeta-command-runner.exe"),
        ])
    );
    assert_eq!(
        context.executable_candidates(ManagedExecutable::WindowsSandboxSetup),
        ExecutableCandidates::SearchPaths(vec![
            resources_directory.join("zeta-windows-sandbox-setup.exe"),
            search_directory.join("zeta-windows-sandbox-setup.exe"),
        ])
    );
    assert_eq!(context.bundled_resource("../bin/zeta"), None);
    assert_eq!(context.bundled_resource(&executable), None);
    assert_eq!(context.bundled_resource(""), None);
    assert_eq!(context.bundled_resource_directory("."), None);
}

#[test]
fn package_directories_without_metadata_are_not_treated_as_an_install() {
    let directory = TestDirectory::new();
    let package = directory.path().join("package");
    let binary_directory = package.join("bin");
    fs::create_dir_all(package.join("zeta-path")).unwrap();
    fs::create_dir_all(package.join("zeta-resources")).unwrap();
    fs::create_dir_all(&binary_directory).unwrap();
    let executable = binary_directory.join("zeta");

    let context = InstallContext::detect(Some(&executable), None, None, None, None, None);

    assert_eq!(context.method(), InstallMethod::Other);
    assert_eq!(context.package_layout(), None);
}

#[test]
fn explicit_override_is_authoritative_and_excludes_fallback_candidates() {
    let directory = TestDirectory::new();
    let executable_directory = directory.path().join("dev");
    let search_directory = directory.path().join("system-bin");
    let executable = executable_directory.join("zeta");
    let override_path = directory.path().join("custom-rg");
    let bubblewrap_override = directory.path().join("custom-bwrap");
    let runner_override = directory.path().join("custom-runner.exe");
    let setup_override = directory.path().join("custom-setup.exe");
    let context = InstallContext::detect(
        Some(&executable),
        Some(override_path.clone().into_os_string()),
        Some(bubblewrap_override.clone().into_os_string()),
        Some(runner_override.clone().into_os_string()),
        Some(setup_override.clone().into_os_string()),
        Some(env::join_paths([&search_directory]).unwrap()),
    );

    let candidates = context.executable_candidates(ManagedExecutable::Ripgrep);

    assert_eq!(context.method(), InstallMethod::Other);
    assert_eq!(
        candidates,
        ExecutableCandidates::ExplicitOverride(ExecutableOverride {
            variable: RIPGREP_OVERRIDE,
            path: override_path,
        })
    );
    assert_eq!(
        context.executable_candidates(ManagedExecutable::Bubblewrap),
        ExecutableCandidates::ExplicitOverride(ExecutableOverride {
            variable: BUBBLEWRAP_OVERRIDE,
            path: bubblewrap_override,
        })
    );
    assert_eq!(
        context.executable_candidates(ManagedExecutable::WindowsCommandRunner),
        ExecutableCandidates::ExplicitOverride(ExecutableOverride {
            variable: WINDOWS_COMMAND_RUNNER_OVERRIDE,
            path: runner_override,
        })
    );
    assert_eq!(
        context.executable_candidates(ManagedExecutable::WindowsSandboxSetup),
        ExecutableCandidates::ExplicitOverride(ExecutableOverride {
            variable: WINDOWS_SANDBOX_SETUP_OVERRIDE,
            path: setup_override,
        })
    );
}

#[test]
fn host_path_candidates_validate_names_and_use_the_frozen_search_path() {
    let first = TestDirectory::new();
    let second = TestDirectory::new();
    let search_path = env::join_paths([first.path(), second.path()]).expect("search path");
    let context = InstallContext::detect(None, None, None, None, None, Some(search_path));
    let name = HostExecutableName::new("rust-analyzer").expect("name");
    let candidates = context.host_path_candidates(&name);

    assert!(candidates.iter().any(|path| path.starts_with(first.path())));
    assert!(
        candidates
            .iter()
            .any(|path| path.starts_with(second.path()))
    );
    assert!(HostExecutableName::new("../rust-analyzer").is_err());
    assert!(HostExecutableName::new("").is_err());
}

fn expected_ripgrep_candidates<'a>(
    directories: impl IntoIterator<Item = &'a PathBuf>,
) -> Vec<PathBuf> {
    let names: &[&str] = if cfg!(windows) {
        &["rg.exe", "rg"]
    } else {
        &["rg"]
    };
    directories
        .into_iter()
        .flat_map(|directory| names.iter().map(|name| directory.join(name)))
        .collect()
}

static NEXT_DIRECTORY: AtomicUsize = AtomicUsize::new(0);

struct TestDirectory {
    path: PathBuf,
}

impl TestDirectory {
    fn new() -> Self {
        let sequence = NEXT_DIRECTORY.fetch_add(1, Ordering::Relaxed);
        let path = env::temp_dir().join(format!(
            "zeta-install-context-tests-{}-{sequence}",
            std::process::id()
        ));
        fs::create_dir_all(&path).unwrap();
        Self { path }
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TestDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}
