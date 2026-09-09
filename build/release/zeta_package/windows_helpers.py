"""Build or validate the first-party Windows sandbox helper executables."""

import hashlib
import subprocess
from dataclasses import dataclass
from pathlib import Path
from typing import Optional

from .cargo import validate_input_binary
from .cargo_paths import cargo_profile_directory
from .cargo_paths import resolve_cargo_target_directory
from build.lib.zeta_build.targets import TargetSpec


COMMAND_RUNNER_NAME = "zeta-command-runner.exe"
SANDBOX_SERVICE_NAME = "zeta-windows-sandbox-service.exe"
SANDBOX_WORKER_NAME = "zeta-windows-sandbox-worker.exe"


@dataclass(frozen=True)
class WindowsSandboxHelpers:
    command_runner: Path
    sandbox_service: Path
    sandbox_worker: Path
    source: str
    command_runner_sha256: str
    sandbox_service_sha256: str
    sandbox_worker_sha256: str


def resolve_windows_sandbox_helpers(
    repository_root: Path,
    spec: TargetSpec,
    command_runner: Optional[Path],
    sandbox_service: Optional[Path],
    sandbox_worker: Optional[Path],
    cargo: str,
    cargo_profile: str,
) -> Optional[WindowsSandboxHelpers]:
    if not spec.is_windows:
        if (
            command_runner is not None
            or sandbox_service is not None
            or sandbox_worker is not None
        ):
            raise RuntimeError(
                "Windows sandbox helper overrides are only supported for Windows packages"
            )
        return None

    if all(
        executable is not None
        for executable in (command_runner, sandbox_service, sandbox_worker)
    ):
        source = "local-override"
    elif all(
        executable is None
        for executable in (command_runner, sandbox_service, sandbox_worker)
    ):
        source = "cargo-build"
    else:
        source = "mixed"
    if any(
        executable is None
        for executable in (command_runner, sandbox_service, sandbox_worker)
    ):
        built_runner, built_service, built_worker = build_windows_sandbox_helpers(
            repository_root,
            spec,
            cargo,
            cargo_profile,
        )
        command_runner = command_runner or built_runner
        sandbox_service = sandbox_service or built_service
        sandbox_worker = sandbox_worker or built_worker

    runner = validate_input_binary(
        command_runner,
        "Windows sandbox command runner",
        "--windows-command-runner-bin",
        True,
    )
    service = validate_input_binary(
        sandbox_service,
        "Windows sandbox service",
        "--windows-sandbox-service-bin",
        True,
    )
    worker = validate_input_binary(
        sandbox_worker,
        "Windows sandbox provisioning worker",
        "--windows-sandbox-worker-bin",
        True,
    )
    return WindowsSandboxHelpers(
        command_runner=runner,
        sandbox_service=service,
        sandbox_worker=worker,
        source=source,
        command_runner_sha256=sha256(runner),
        sandbox_service_sha256=sha256(service),
        sandbox_worker_sha256=sha256(worker),
    )


def build_windows_sandbox_helpers(
    repository_root: Path,
    spec: TargetSpec,
    cargo: str,
    cargo_profile: str,
) -> tuple[Path, Path, Path]:
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
    subprocess.run(
        [
            cargo,
            "build",
            "--manifest-path",
            str(rust_workspace / "Cargo.toml"),
            "--package",
            "zeta-windows-sandbox-service",
            "--bin",
            "zeta-windows-sandbox-service",
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
    return (
        output / COMMAND_RUNNER_NAME,
        output / SANDBOX_SERVICE_NAME,
        output / SANDBOX_WORKER_NAME,
    )


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with open(path, "rb") as input_file:
        for block in iter(lambda: input_file.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()
