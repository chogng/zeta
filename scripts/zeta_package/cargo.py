"""Build or validate the first-party Zeta package entrypoint."""

import os
import stat
import subprocess
from pathlib import Path
from typing import Optional

from .targets import TargetSpec


def resolve_zeta_binary(
    repository_root: Path,
    spec: TargetSpec,
    explicit_binary: Optional[Path],
    cargo: str,
    cargo_profile: str,
) -> Path:
    if explicit_binary is not None:
        return validate_input_binary(
            explicit_binary, "Zeta executable", "--zeta-bin", spec.is_windows
        )

    rust_workspace = repository_root
    target_directory = repository_root / "target"
    command = [
        cargo,
        "build",
        "--manifest-path",
        str(repository_root / "Cargo.toml"),
        "--package",
        "zeta-cli",
        "--bin",
        "zeta",
        "--profile",
        cargo_profile,
        "--target",
        spec.target,
        "--target-dir",
        str(target_directory),
    ]
    subprocess.run(command, check=True)
    profile_directory = "debug" if cargo_profile == "dev" else cargo_profile
    binary = (
        target_directory
        / spec.target
        / profile_directory
        / ("zeta" + spec.executable_suffix)
    )
    return validate_input_binary(binary, "built Zeta executable", cargo, spec.is_windows)


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


def is_executable(path: Path) -> bool:
    mode = path.stat().st_mode
    return bool(mode & (stat.S_IXUSR | stat.S_IXGRP | stat.S_IXOTH)) and os.access(
        str(path), os.X_OK
    )
