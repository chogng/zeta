use std::path::PathBuf;
use std::process::ExitCode;

use zui::distribution::BundleBuilder;
use zui::distribution::BundleManifest;
use zui::distribution::BundleTarget;
use zui::distribution::InstallerBuilder;
use zui::distribution::InstallerTarget;
use zui::distribution::SystemInstallerTool;

fn main() -> ExitCode {
    match run(std::env::args_os().skip(1).collect()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("zui-packager: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run(arguments: Vec<std::ffi::OsString>) -> Result<(), String> {
    if arguments.len() < 3 || arguments.len() > 4 {
        return Err(
            "usage: zui-packager <bundle|installer> <manifest.json> <output-directory> [macos|linux|windows]"
                .to_owned(),
        );
    }
    let operation = arguments[0].to_string_lossy();
    if operation != "bundle" && operation != "installer" {
        return Err("first argument must be `bundle` or `installer`".to_owned());
    }
    let manifest_path = PathBuf::from(&arguments[1]);
    let output_directory = PathBuf::from(&arguments[2]);
    let target = arguments
        .get(3)
        .map(|target| parse_target(target.to_string_lossy().as_ref()))
        .transpose()?
        .unwrap_or_else(current_targets);
    let bytes = std::fs::read(&manifest_path)
        .map_err(|error| format!("could not read {}: {error}", manifest_path.display()))?;
    let manifest_directory = manifest_path
        .parent()
        .unwrap_or_else(|| std::path::Path::new("."));
    let manifest = BundleManifest::from_json(&bytes)
        .map_err(|error| error.to_string())?
        .resolve_paths_from(manifest_directory);
    let bundle = BundleBuilder::build(&manifest, target.0, &output_directory)
        .map_err(|error| error.to_string())?;
    if operation == "bundle" {
        println!("{}", bundle.root.display());
        return Ok(());
    }
    let installer = InstallerBuilder::build(
        &manifest,
        &bundle,
        target.1,
        output_directory,
        &SystemInstallerTool,
    )
    .map_err(|error| error.to_string())?;
    println!("{}", installer.artifact.display());
    Ok(())
}

fn parse_target(value: &str) -> Result<(BundleTarget, InstallerTarget), String> {
    match value {
        "macos" => Ok((
            BundleTarget::MacOsApplication,
            InstallerTarget::MacOsPackage,
        )),
        "linux" => Ok((BundleTarget::LinuxAppDir, InstallerTarget::LinuxAppImage)),
        "windows" => Ok((BundleTarget::WindowsPortable, InstallerTarget::WindowsMsi)),
        _ => Err(format!("unsupported bundle target `{value}`")),
    }
}

fn current_targets() -> (BundleTarget, InstallerTarget) {
    (BundleTarget::current(), InstallerTarget::current())
}
