"""Canonical Zeta package directory assembly and validation."""

import hashlib
import json
import os
import re
import shutil
import stat
import tempfile
from pathlib import Path
from typing import Dict, Optional

from .bubblewrap import BubblewrapResolution
from .node import NodeResolution
from .ripgrep import RipgrepResolution
from .targets import TargetSpec
from .windows_helpers import (
    COMMAND_RUNNER_NAME,
    SANDBOX_SETUP_NAME,
    WindowsSandboxHelpers,
)


LAYOUT_VERSION = 2
METADATA_FILE = "zeta-package.json"
SKILL_NAME = re.compile(r"^[a-z0-9]+(?:-[a-z0-9]+)*$")


def build_package_directory(
    output: Path,
    repository_root: Path,
    version: str,
    spec: TargetSpec,
    server_binary: Path,
    app_server_daemon_binary: Path,
    code_mode_host_binary: Path,
    ripgrep: RipgrepResolution,
    node: Optional[NodeResolution],
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
        extensions_directory = staging / "zeta-resources" / "extensions"
        product_services_directory = staging / "zeta-resources" / "product-services"
        license_directory = staging / "zeta-resources" / "licenses" / "ripgrep"
        vscode_license_directory = staging / "zeta-resources" / "licenses" / "vscode"
        binary_directory.mkdir()
        path_directory.mkdir()
        license_directory.mkdir(parents=True)
        vscode_license_directory.mkdir()
        copy_builtin_skills(
            repository_root / "zeta-rs" / "skills" / "assets",
            skills_directory,
        )
        copy_builtin_extensions(
            repository_root / "extensions",
            extensions_directory,
        )
        copy_regular_tree(
            repository_root / "resources" / "product-services",
            product_services_directory,
            "product services",
        )

        copy_executable(
            server_binary,
            binary_directory / spec.server_name,
            is_windows=spec.is_windows,
        )
        copy_executable(
            app_server_daemon_binary,
            binary_directory / spec.app_server_daemon_name,
            is_windows=spec.is_windows,
        )
        copy_executable(
            code_mode_host_binary,
            binary_directory / spec.code_mode_host_name,
            is_windows=spec.is_windows,
        )
        copy_executable(
            ripgrep.executable,
            path_directory / spec.ripgrep_name,
            is_windows=spec.is_windows,
        )
        if node is not None:
            node_directory = staging / "zeta-resources" / "node" / "bin"
            node_license_directory = staging / "zeta-resources" / "licenses" / "node"
            node_directory.mkdir(parents=True)
            node_license_directory.mkdir(parents=True)
            copy_executable(
                node.executable,
                node_directory / spec.node_name,
                is_windows=spec.is_windows,
            )
            shutil.copyfile(node.license_file, node_license_directory / "LICENSE")
        for name in ("LICENSE-MIT", "UNLICENSE"):
            shutil.copyfile(
                repository_root / "third_party" / "ripgrep" / name,
                license_directory / name,
            )
        shutil.copyfile(
            repository_root / "third_party" / "vscode" / "LICENSE.txt",
            vscode_license_directory / "LICENSE.txt",
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
        components = {
            "appServerDaemon": {
                "source": "cargo-build",
                "binarySha256": file_sha256(
                    binary_directory / spec.app_server_daemon_name
                ),
            },
            "codeModeHost": {
                "source": "cargo-build",
                "binarySha256": file_sha256(
                    binary_directory / spec.code_mode_host_name
                ),
            },
            "ripgrep": ripgrep_metadata,
            "serverHost": {
                "source": "cargo-build",
                "binarySha256": file_sha256(binary_directory / spec.server_name),
            },
        }
        if node is not None:
            components["node"] = {
                "version": node.version,
                "source": node.source,
                "binarySha256": node.binary_sha256,
                "archive": node.archive,
                "archiveSha256": node.archive_sha256,
            }
        if bubblewrap_metadata is not None:
            components["bubblewrap"] = bubblewrap_metadata
        if windows_sandbox_metadata is not None:
            components["windowsSandbox"] = windows_sandbox_metadata
        protocol = load_protocol_metadata(repository_root)
        build_identity = {
            "appServerDaemonSha256": components["appServerDaemon"]["binarySha256"],
            "codeModeHostSha256": components["codeModeHost"]["binarySha256"],
            "protocol": protocol,
            "serverHostSha256": components["serverHost"]["binarySha256"],
            "target": spec.target,
            "version": version,
        }
        metadata = {
            "buildId": "sha256:"
            + hashlib.sha256(
                json.dumps(
                    build_identity, separators=(",", ":"), sort_keys=True
                ).encode("utf-8")
            ).hexdigest(),
            "layoutVersion": LAYOUT_VERSION,
            "version": version,
            "target": spec.target,
            "entrypoint": "bin/" + spec.server_name,
            "pathDir": "zeta-path",
            "resourcesDir": "zeta-resources",
            "javascriptRuntime": {
                "kind": "packagedNode" if node is not None else "hostProvidedNode",
            },
            "components": components,
            "protocol": protocol,
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
        "entrypoint": "bin/" + spec.server_name,
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

    executables = [
        package / "bin" / spec.server_name,
        package / "bin" / spec.app_server_daemon_name,
        package / "bin" / spec.code_mode_host_name,
        package / "zeta-path" / spec.ripgrep_name,
    ]
    components = metadata.get("components")
    if not isinstance(components, dict):
        raise RuntimeError("Invalid package component metadata")
    first_party_artifacts = {
        "appServerDaemon": package / "bin" / spec.app_server_daemon_name,
        "codeModeHost": package / "bin" / spec.code_mode_host_name,
        "serverHost": package / "bin" / spec.server_name,
    }
    for component_name, artifact in first_party_artifacts.items():
        component = components.get(component_name)
        expected_digest = (
            component.get("binarySha256") if isinstance(component, dict) else None
        )
        if (
            not isinstance(expected_digest, str)
            or re.fullmatch(r"[a-f0-9]{64}", expected_digest) is None
            or file_sha256(artifact) != expected_digest
        ):
            raise RuntimeError(
                "Package component digest does not match: {}".format(component_name)
            )
    build_identity = {
        "appServerDaemonSha256": components["appServerDaemon"]["binarySha256"],
        "codeModeHostSha256": components["codeModeHost"]["binarySha256"],
        "protocol": metadata.get("protocol"),
        "serverHostSha256": components["serverHost"]["binarySha256"],
        "target": metadata.get("target"),
        "version": metadata.get("version"),
    }
    expected_build_id = "sha256:" + hashlib.sha256(
        json.dumps(build_identity, separators=(",", ":"), sort_keys=True).encode(
            "utf-8"
        )
    ).hexdigest()
    if metadata.get("buildId") != expected_build_id:
        raise RuntimeError(
            "Package build identity does not match its first-party artifacts"
        )
    javascript_runtime = metadata.get("javascriptRuntime")
    if javascript_runtime == {"kind": "packagedNode"}:
        if not isinstance(components.get("node"), dict):
            raise RuntimeError("Packaged Node runtime metadata is missing")
        executables.append(
            package / "zeta-resources" / "node" / "bin" / spec.node_name
        )
    elif javascript_runtime == {"kind": "hostProvidedNode"}:
        if "node" in components:
            raise RuntimeError("Host-provided runtime package contains Node metadata")
        if (package / "zeta-resources" / "node").exists():
            raise RuntimeError("Host-provided runtime package contains a Node executable")
        if (package / "zeta-resources" / "licenses" / "node").exists():
            raise RuntimeError("Host-provided runtime package contains a Node license")
    else:
        raise RuntimeError("Invalid package JavaScript runtime declaration")
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
    vscode_license = (
        package
        / "zeta-resources"
        / "licenses"
        / "vscode"
        / "LICENSE.txt"
    )
    if vscode_license.is_symlink() or not vscode_license.is_file():
        raise RuntimeError("Missing VS Code extension license: {}".format(vscode_license))
    if javascript_runtime == {"kind": "packagedNode"}:
        node_license = package / "zeta-resources" / "licenses" / "node" / "LICENSE"
        if node_license.is_symlink() or not node_license.is_file():
            raise RuntimeError("Missing Node.js license: {}".format(node_license))
    validate_builtin_skills(package / "zeta-resources" / "skills")
    validate_builtin_extensions(package / "zeta-resources" / "extensions")
    validate_product_services(package / "zeta-resources" / "product-services")
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


def copy_builtin_extensions(source: Path, destination: Path) -> None:
    if source.is_symlink() or not source.is_dir():
        raise RuntimeError(
            "Built-in extension source is not a real directory: {}".format(source)
        )
    extension_entries = [
        child
        for child in sorted(source.iterdir(), key=lambda path: path.name)
        if child.name not in ("README.md", "BUILD.bazel")
    ]
    if not extension_entries:
        raise RuntimeError("Built-in extension source is empty: {}".format(source))
    destination.mkdir(parents=True)
    for extension_directory in extension_entries:
        if extension_directory.is_symlink() or not extension_directory.is_dir():
            raise RuntimeError(
                "Invalid built-in extension package: {}".format(extension_directory)
            )
        manifest = extension_directory / "package.json"
        if manifest.is_symlink() or not manifest.is_file():
            raise RuntimeError(
                "Built-in extension is missing package.json: {}".format(
                    extension_directory
                )
            )
        copy_regular_tree(
            extension_directory,
            destination / extension_directory.name,
            "extension package",
        )


def copy_regular_tree(source: Path, destination: Path, kind: str = "Skill") -> None:
    destination.mkdir()
    for child in sorted(source.iterdir(), key=lambda path: path.name):
        metadata = child.lstat()
        target = destination / child.name
        if child.is_symlink():
            raise RuntimeError(
                "Built-in {} asset is a symbolic link: {}".format(kind, child)
            )
        if stat.S_ISDIR(metadata.st_mode):
            copy_regular_tree(child, target, kind)
        elif stat.S_ISREG(metadata.st_mode):
            if metadata.st_nlink > 1:
                raise RuntimeError(
                    "Built-in {} asset is a hard link: {}".format(kind, child)
                )
            shutil.copyfile(child, target)
        else:
            raise RuntimeError(
                "Built-in {} asset is not a regular file or directory: {}".format(
                    kind, child
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


def validate_builtin_extensions(extensions_directory: Path) -> None:
    if extensions_directory.is_symlink() or not extensions_directory.is_dir():
        raise RuntimeError("Package is missing built-in extensions")
    extension_directories = sorted(
        extensions_directory.iterdir(), key=lambda path: path.name
    )
    if not extension_directories:
        raise RuntimeError("Package contains no built-in extensions")
    for extension_directory in extension_directories:
        if (
            extension_directory.is_symlink()
            or not extension_directory.is_dir()
            or not (extension_directory / "package.json").is_file()
        ):
            raise RuntimeError(
                "Package contains an invalid built-in extension: {}".format(
                    extension_directory
                )
            )


def validate_product_services(product_services_directory: Path) -> None:
    if product_services_directory.is_symlink() or not product_services_directory.is_dir():
        raise RuntimeError("Package is missing product services")
    config_path = product_services_directory / "product-services.json"
    root_path = product_services_directory / "marketplace-root.json"
    for path in (config_path, root_path):
        if path.is_symlink() or not path.is_file():
            raise RuntimeError("Package is missing product service file: {}".format(path))
    document = json.loads(config_path.read_text(encoding="utf-8"))
    marketplace_manager = document.get("marketplaceManager")
    if document.get("schemaVersion") != 1 or not isinstance(marketplace_manager, dict):
        raise RuntimeError("Package product services configuration is invalid")
    if marketplace_manager.get("trustedRoot") != "marketplace-root.json":
        raise RuntimeError("Package product services does not pin the Zeta Marketplace root")


def copy_executable(source: Path, destination: Path, is_windows: bool) -> None:
    shutil.copyfile(source, destination)
    if not is_windows:
        mode = destination.stat().st_mode
        destination.chmod(mode | stat.S_IXUSR | stat.S_IXGRP | stat.S_IXOTH)


def is_executable(path: Path) -> bool:
    if os.name == "nt":
        return True
    return bool(path.stat().st_mode & stat.S_IXUSR)


def write_json(path: Path, value: object) -> None:
    path.write_text(json.dumps(value, indent=2) + "\n", encoding="utf-8")


def file_sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def load_protocol_metadata(repository_root: Path) -> Dict[str, object]:
    generated = (
        repository_root / "zeta-rs" / "app-server-protocol" / "schema" / "types.ts"
    ).read_text(encoding="utf-8")
    major = re.search(
        r"^export const APP_SERVER_PROTOCOL_MAJOR = (\d+) as const;$",
        generated,
        re.MULTILINE,
    )
    revision = re.search(
        r"^export const APP_SERVER_PROTOCOL_REVISION = (\d+) as const;$",
        generated,
        re.MULTILINE,
    )
    schema_hash = re.search(
        r'^export const APP_SERVER_SCHEMA_HASH = "(sha256:[a-f0-9]{64})" as const;$',
        generated,
        re.MULTILINE,
    )
    if major is None or revision is None or schema_hash is None:
        raise RuntimeError("Generated App Server protocol metadata is invalid")
    return {
        "major": int(major.group(1)),
        "revision": int(revision.group(1)),
        "schemaHash": schema_hash.group(1),
    }
