"""Canonical Zeta package directory assembly and validation."""

import json
import re
import shutil
import stat
import tempfile
from pathlib import Path
from typing import Optional

from .bubblewrap import BubblewrapResolution
from .ripgrep import RipgrepResolution
from .targets import TargetSpec
from .windows_helpers import (
    COMMAND_RUNNER_NAME,
    SANDBOX_SETUP_NAME,
    WindowsSandboxHelpers,
)


LAYOUT_VERSION = 1
METADATA_FILE = "zeta-package.json"
SKILL_NAME = re.compile(r"^[a-z0-9]+(?:-[a-z0-9]+)*$")


def build_package_directory(
    output: Path,
    repository_root: Path,
    version: str,
    spec: TargetSpec,
    zeta_binary: Path,
    ripgrep: RipgrepResolution,
    bubblewrap: Optional[BubblewrapResolution] = None,
    windows_helpers: Optional[WindowsSandboxHelpers] = None,
) -> None:
    output = output.expanduser().resolve()
    if output.exists():
        raise RuntimeError("Refusing to replace existing package output: {}".format(output))
    output.parent.mkdir(parents=True, exist_ok=True)
    staging = Path(
        tempfile.mkdtemp(prefix="." + output.name + ".partial-", dir=str(output.parent))
    )
    try:
        binary_directory = staging / "bin"
        path_directory = staging / "zeta-path"
        skills_directory = staging / "zeta-resources" / "skills"
        license_directory = staging / "zeta-resources" / "licenses" / "ripgrep"
        binary_directory.mkdir()
        path_directory.mkdir()
        license_directory.mkdir(parents=True)
        copy_builtin_skills(
            repository_root / "zeta-rs" / "skills" / "assets",
            skills_directory,
        )

        copy_executable(
            zeta_binary,
            binary_directory / spec.zeta_name,
            is_windows=spec.is_windows,
        )
        copy_executable(
            ripgrep.executable,
            path_directory / spec.ripgrep_name,
            is_windows=spec.is_windows,
        )
        for name in ("LICENSE-MIT", "UNLICENSE"):
            shutil.copyfile(
                repository_root / "third_party" / "ripgrep" / name,
                license_directory / name,
            )

        bubblewrap_metadata = None
        if bubblewrap is not None:
            copy_executable(
                bubblewrap.executable,
                staging / "zeta-resources" / "bwrap",
                is_windows=False,
            )
            bubblewrap_license_directory = (
                staging / "zeta-resources" / "licenses" / "bubblewrap"
            )
            bubblewrap_license_directory.mkdir()
            for license_file in bubblewrap.license_files:
                shutil.copyfile(
                    license_file,
                    bubblewrap_license_directory / license_file.name,
                )
            bubblewrap_metadata = {
                "version": bubblewrap.version,
                "source": bubblewrap.source,
                "binarySha256": bubblewrap.binary_sha256,
                "sourceArchive": bubblewrap.source_archive,
                "sourceArchiveSha256": bubblewrap.source_archive_sha256,
            }

        windows_sandbox_metadata = None
        if windows_helpers is not None:
            resources_directory = staging / "zeta-resources"
            copy_executable(
                windows_helpers.command_runner,
                resources_directory / COMMAND_RUNNER_NAME,
                is_windows=True,
            )
            copy_executable(
                windows_helpers.sandbox_setup,
                resources_directory / SANDBOX_SETUP_NAME,
                is_windows=True,
            )
            windows_sandbox_metadata = {
                "source": windows_helpers.source,
                "commandRunnerSha256": windows_helpers.command_runner_sha256,
                "sandboxSetupSha256": windows_helpers.sandbox_setup_sha256,
            }

        ripgrep_metadata = {
            "version": ripgrep.version,
            "source": ripgrep.source,
            "binarySha256": ripgrep.binary_sha256,
        }
        if ripgrep.archive is not None:
            ripgrep_metadata["archive"] = ripgrep.archive
        if ripgrep.archive_sha256 is not None:
            ripgrep_metadata["archiveSha256"] = ripgrep.archive_sha256
        components = {"ripgrep": ripgrep_metadata}
        if bubblewrap_metadata is not None:
            components["bubblewrap"] = bubblewrap_metadata
        if windows_sandbox_metadata is not None:
            components["windowsSandbox"] = windows_sandbox_metadata
        metadata = {
            "layoutVersion": LAYOUT_VERSION,
            "version": version,
            "target": spec.target,
            "entrypoint": "bin/" + spec.zeta_name,
            "pathDir": "zeta-path",
            "resourcesDir": "zeta-resources",
            "components": components,
        }
        write_json(staging / METADATA_FILE, metadata)
        validate_package_directory(staging, spec)
        staging.rename(output)
    except Exception:
        shutil.rmtree(staging, ignore_errors=True)
        raise


def validate_package_directory(package: Path, spec: TargetSpec) -> None:
    required_directories = (
        package / "bin",
        package / "zeta-path",
        package / "zeta-resources",
    )
    for directory in required_directories:
        if not directory.is_dir():
            raise RuntimeError("Missing package directory: {}".format(directory))

    metadata_path = package / METADATA_FILE
    if not metadata_path.is_file():
        raise RuntimeError("Missing package metadata: {}".format(metadata_path))
    metadata = json.loads(metadata_path.read_text(encoding="utf-8"))
    expected = {
        "layoutVersion": LAYOUT_VERSION,
        "target": spec.target,
        "entrypoint": "bin/" + spec.zeta_name,
        "pathDir": "zeta-path",
        "resourcesDir": "zeta-resources",
    }
    for key, expected_value in expected.items():
        if metadata.get(key) != expected_value:
            raise RuntimeError(
                "Invalid package metadata {!r}: expected {!r}, got {!r}".format(
                    key, expected_value, metadata.get(key)
                )
            )

    executables = (
        package / "bin" / spec.zeta_name,
        package / "zeta-path" / spec.ripgrep_name,
    )
    for executable in executables:
        if not executable.is_file():
            raise RuntimeError("Missing package executable: {}".format(executable))
        if not spec.is_windows and not is_executable(executable):
            raise RuntimeError("Package file is not executable: {}".format(executable))
    for license_name in ("LICENSE-MIT", "UNLICENSE"):
        license_path = (
            package
            / "zeta-resources"
            / "licenses"
            / "ripgrep"
            / license_name
        )
        if not license_path.is_file():
            raise RuntimeError("Missing ripgrep license: {}".format(license_path))
    validate_builtin_skills(package / "zeta-resources" / "skills")
    if spec.is_linux:
        bubblewrap = package / "zeta-resources" / "bwrap"
        if not bubblewrap.is_file() or not is_executable(bubblewrap):
            raise RuntimeError(
                "Linux package is missing executable zeta-resources/bwrap"
            )
        for license_name in ("COPYING",):
            license_path = (
                package
                / "zeta-resources"
                / "licenses"
                / "bubblewrap"
                / license_name
            )
            if not license_path.is_file():
                raise RuntimeError(
                    "Missing Bubblewrap license: {}".format(license_path)
                )
    if spec.is_windows:
        for helper_name in (COMMAND_RUNNER_NAME, SANDBOX_SETUP_NAME):
            helper = package / "zeta-resources" / helper_name
            if not helper.is_file():
                raise RuntimeError(
                    "Windows package is missing sandbox helper {}".format(helper_name)
                )


def copy_builtin_skills(source: Path, destination: Path) -> None:
    if source.is_symlink() or not source.is_dir():
        raise RuntimeError("Built-in Skill source is not a real directory: {}".format(source))
    skill_directories = [
        child
        for child in sorted(source.iterdir(), key=lambda path: path.name)
        if child.name != "BUILD.bazel"
    ]
    if not skill_directories:
        raise RuntimeError("Built-in Skill source is empty: {}".format(source))
    destination.mkdir(parents=True)
    for skill_directory in skill_directories:
        if (
            skill_directory.is_symlink()
            or not skill_directory.is_dir()
            or SKILL_NAME.fullmatch(skill_directory.name) is None
        ):
            raise RuntimeError(
                "Invalid built-in Skill directory: {}".format(skill_directory)
            )
        if not (skill_directory / "SKILL.md").is_file():
            raise RuntimeError(
                "Built-in Skill is missing SKILL.md: {}".format(skill_directory)
            )
        copy_regular_tree(skill_directory, destination / skill_directory.name)


def copy_regular_tree(source: Path, destination: Path) -> None:
    destination.mkdir()
    for child in sorted(source.iterdir(), key=lambda path: path.name):
        metadata = child.lstat()
        target = destination / child.name
        if child.is_symlink():
            raise RuntimeError("Built-in Skill asset is a symbolic link: {}".format(child))
        if stat.S_ISDIR(metadata.st_mode):
            copy_regular_tree(child, target)
        elif stat.S_ISREG(metadata.st_mode):
            if metadata.st_nlink > 1:
                raise RuntimeError("Built-in Skill asset is a hard link: {}".format(child))
            shutil.copyfile(child, target)
        else:
            raise RuntimeError(
                "Built-in Skill asset is not a regular file or directory: {}".format(
                    child
                )
            )


def validate_builtin_skills(skills_directory: Path) -> None:
    if skills_directory.is_symlink() or not skills_directory.is_dir():
        raise RuntimeError("Package is missing built-in Skills")
    skill_directories = sorted(skills_directory.iterdir(), key=lambda path: path.name)
    if not skill_directories:
        raise RuntimeError("Package contains no built-in Skills")
    for skill_directory in skill_directories:
        if (
            skill_directory.is_symlink()
            or not skill_directory.is_dir()
            or SKILL_NAME.fullmatch(skill_directory.name) is None
            or not (skill_directory / "SKILL.md").is_file()
        ):
            raise RuntimeError(
                "Package contains an invalid built-in Skill: {}".format(
                    skill_directory
                )
            )


def copy_executable(source: Path, destination: Path, is_windows: bool) -> None:
    shutil.copyfile(source, destination)
    if not is_windows:
        mode = destination.stat().st_mode
        destination.chmod(mode | stat.S_IXUSR | stat.S_IXGRP | stat.S_IXOTH)


def is_executable(path: Path) -> bool:
    return bool(path.stat().st_mode & stat.S_IXUSR)


def write_json(path: Path, value: object) -> None:
    path.write_text(json.dumps(value, indent=2) + "\n", encoding="utf-8")
