"""Build or validate the first-party Windows sandbox helper executables."""

import hashlib
import subprocess
from dataclasses import dataclass
from pathlib import Path
from typing import Optional

from .cargo import validate_input_binary
from .cargo_paths import cargo_profile_directory
from .cargo_paths import resolve_cargo_target_directory
from .targets import TargetSpec


COMMAND_RUNNER_NAME = "zeta-command-runner.exe"
SANDBOX_SETUP_NAME = "zeta-windows-sandbox-setup.exe"


@dataclass(frozen=True)
class WindowsSandboxHelpers:
    command_runner: Path
    sandbox_setup: Path
    source: str
    command_runner_sha256: str
    sandbox_setup_sha256: str


def resolve_windows_sandbox_helpers(
    repository_root: Path,
    spec: TargetSpec,
    command_runner: Optional[Path],
    sandbox_setup: Optional[Path],
    cargo: str,
    cargo_profile: str,
) -> Optional[WindowsSandboxHelpers]:
    if not spec.is_windows:
        if command_runner is not None or sandbox_setup is not None:
            raise RuntimeError(
                "Windows sandbox helper overrides are only supported for Windows packages"
            )
        return None

    if command_runner is not None and sandbox_setup is not None:
        source = "local-override"
    elif command_runner is None and sandbox_setup is None:
        source = "cargo-build"
    else:
        source = "mixed"
    if command_runner is None or sandbox_setup is None:
        built_runner, built_setup = build_windows_sandbox_helpers(
            repository_root,
            spec,
            cargo,
            cargo_profile,
        )
        command_runner = command_runner or built_runner
        sandbox_setup = sandbox_setup or built_setup

    runner = validate_input_binary(
        command_runner,
        "Windows sandbox command runner",
        "--windows-command-runner-bin",
        True,
    )
    setup = validate_input_binary(
        sandbox_setup,
        "Windows sandbox setup helper",
        "--windows-sandbox-setup-bin",
        True,
    )
    return WindowsSandboxHelpers(
        command_runner=runner,
        sandbox_setup=setup,
        source=source,
        command_runner_sha256=sha256(runner),
        sandbox_setup_sha256=sha256(setup),
    )


def build_windows_sandbox_helpers(
    repository_root: Path,
    spec: TargetSpec,
    cargo: str,
    cargo_profile: str,
) -> tuple[Path, Path]:
    rust_workspace = repository_root
    target_directory = resolve_cargo_target_directory(rust_workspace)
    subprocess.run(
        [
            cargo,
            "build",
            "--manifest-path",
            str(rust_workspace / "Cargo.toml"),
            "--package",
            "zeta-windows-sandbox",
            "--bins",
            "--profile",
            cargo_profile,
            "--target",
            spec.target,
            "--target-dir",
            str(target_directory),
        ],
        check=True,
    )
    profile_directory = cargo_profile_directory(cargo_profile)
    output = target_directory / spec.target / profile_directory
    return output / COMMAND_RUNNER_NAME, output / SANDBOX_SETUP_NAME


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with open(path, "rb") as input_file:
        for block in iter(lambda: input_file.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()
