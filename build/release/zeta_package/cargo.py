"""Build or validate the first-party Zeta package entrypoint."""

import os
import stat
import subprocess
from pathlib import Path
from typing import Optional

from .cargo_paths import cargo_profile_directory
from .cargo_paths import resolve_cargo_target_directory
from build.lib.zeta_build.targets import TargetSpec
from build.lib.zeta_build.v8 import resolve_v8_cargo_env


def cargo_environment(spec: TargetSpec) -> dict[str, str]:
    environment = os.environ.copy()
    environment.update(resolve_v8_cargo_env(spec, environ=environment))
    return environment


def resolve_server_binary(
    repository_root: Path,
    spec: TargetSpec,
    explicit_binary: Optional[Path],
    cargo: str,
    cargo_profile: str,
) -> Path:
    if explicit_binary is not None:
        return validate_input_binary(
            explicit_binary, "Zeta server executable", "--server-bin", spec.is_windows
        )

    rust_workspace = repository_root
    target_directory = resolve_cargo_target_directory(repository_root)
    command = [
        cargo,
        "build",
        "--manifest-path",
        str(repository_root / "Cargo.toml"),
        "--package",
        "zeta-app-server",
        "--bin",
        "zeta-app-server",
        "--profile",
        cargo_profile,
        "--target",
        spec.target,
        "--target-dir",
        str(target_directory),
    ]
    subprocess.run(command, check=True, env=cargo_environment(spec))
    profile_directory = cargo_profile_directory(cargo_profile)
    binary = (
        target_directory
        / spec.target
        / profile_directory
        / ("zeta-app-server" + spec.executable_suffix)
    )
    return validate_input_binary(
        binary, "built Zeta server executable", cargo, spec.is_windows
    )


def resolve_cli_binary(
    spec: TargetSpec,
    explicit_binary: Optional[Path],
) -> Optional[Path]:
    if explicit_binary is None:
        return None
    return validate_input_binary(
        explicit_binary, "Zeta CLI executable", "--cli-bin", spec.is_windows
    )


def resolve_app_server_daemon_binary(
    repository_root: Path,
    spec: TargetSpec,
    explicit_binary: Optional[Path],
    cargo: str,
    cargo_profile: str,
) -> Path:
    if explicit_binary is not None:
        return validate_input_binary(
            explicit_binary,
            "Zeta App Server daemon executable",
            "--app-server-daemon-bin",
            spec.is_windows,
        )

    target_directory = resolve_cargo_target_directory(repository_root)
    command = [
        cargo,
        "build",
        "--manifest-path",
        str(repository_root / "Cargo.toml"),
        "--package",
        "zeta-app-server-daemon",
        "--bin",
        "zeta-app-server-daemon",
        "--profile",
        cargo_profile,
        "--target",
        spec.target,
        "--target-dir",
        str(target_directory),
    ]
    subprocess.run(command, check=True, env=cargo_environment(spec))
    profile_directory = cargo_profile_directory(cargo_profile)
    binary = (
        target_directory / spec.target / profile_directory / spec.app_server_daemon_name
    )
    return validate_input_binary(
        binary,
        "built Zeta App Server daemon executable",
        cargo,
        spec.is_windows,
    )


def resolve_code_mode_host_binary(
    repository_root: Path,
    spec: TargetSpec,
    explicit_binary: Optional[Path],
    cargo: str,
    cargo_profile: str,
) -> Path:
    if explicit_binary is not None:
        return validate_input_binary(
            explicit_binary,
            "Zeta Code Mode Host executable",
            "--code-mode-host-bin",
            spec.is_windows,
        )

    target_directory = resolve_cargo_target_directory(repository_root)
    subprocess.run(
        [
            cargo,
            "build",
            "--manifest-path",
            str(repository_root / "Cargo.toml"),
            "--package",
            "zeta-code-mode-host",
            "--bin",
            "zeta-code-mode-host",
            "--profile",
            cargo_profile,
            "--target",
            spec.target,
            "--target-dir",
            str(target_directory),
        ],
        check=True,
        env=cargo_environment(spec),
    )
    binary = (
        target_directory
        / spec.target
        / cargo_profile_directory(cargo_profile)
        / spec.code_mode_host_name
    )
    return validate_input_binary(
        binary, "built Zeta Code Mode Host executable", cargo, spec.is_windows
    )


def validate_input_binary(
    path: Path, description: str, flag_name: str, is_windows_target: bool
) -> Path:
    resolved = path.expanduser().resolve()
    if not resolved.is_file():
        raise RuntimeError(
            "{} does not exist: {} (source: {})".format(
                description, resolved, flag_name
            )
        )
    if not is_windows_target and not is_executable(resolved):
        raise RuntimeError("{} is not executable: {}".format(description, resolved))
    return resolved


def resolve_windows_sandbox_binary(repository_root, spec, explicit_binary, cargo, cargo_profile):
    if not spec.is_windows:
        if explicit_binary is not None:
            raise RuntimeError("Windows sandbox executable requires a Windows target")
        return None
    if explicit_binary is not None:
        return validate_input_binary(explicit_binary, "Windows sandbox executable", "--windows-sandbox-bin", True)
    target_directory = resolve_cargo_target_directory(repository_root)
    subprocess.run([
        cargo, "build", "--manifest-path", str(repository_root / "Cargo.toml"),
        "--package", "zeta-windows-sandbox", "--bin", "zeta-windows-sandbox", "--locked",
        "--profile", cargo_profile, "--target", spec.target, "--target-dir", str(target_directory),
    ], check=True)
    return validate_input_binary(target_directory / spec.target / cargo_profile_directory(cargo_profile) / "zeta-windows-sandbox.exe", "built Windows sandbox executable", cargo, True)


def is_executable(path: Path) -> bool:
    if os.name == "nt":
        return True
    mode = path.stat().st_mode
    return bool(mode & (stat.S_IXUSR | stat.S_IXGRP | stat.S_IXOTH)) and os.access(
        str(path), os.X_OK
    )


def resolve_remote_binary(repository_root, spec, explicit_binary, cargo, cargo_profile, *, server):
    package, name = ("zeta-remote-server", spec.remote_server_name) if server else ("zeta-remote-connections", spec.remote_name)
    if explicit_binary is not None:
        return validate_input_binary(explicit_binary, name, "--remote-server-bin" if server else "--remote-bin", spec.is_windows)
    target_directory = resolve_cargo_target_directory(repository_root)
    subprocess.run([
        cargo, "build", "--manifest-path", str(repository_root / "Cargo.toml"),
        "--package", package, "--bin", "zeta-remote-server" if server else "zeta-remote",
        "--profile", cargo_profile, "--target", spec.target, "--target-dir", str(target_directory),
    ], check=True, env=cargo_environment(spec))
    return validate_input_binary(target_directory / spec.target / cargo_profile_directory(cargo_profile) / name, name, cargo, spec.is_windows)
