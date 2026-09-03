"""Command-line interface for building a canonical Zeta package directory."""

import argparse
import subprocess
import tempfile
from pathlib import Path
from typing import Dict, Optional, Sequence

from build.lib.zeta_build.targets import TARGETS, default_target

from .bubblewrap import resolve_bubblewrap
from .cargo import (
    resolve_app_server_daemon_binary,
    resolve_code_mode_host_binary,
    resolve_server_binary,
)
from .layout import build_package_directory, load_protocol_metadata
from .node import resolve_node
from .ripgrep import resolve_ripgrep
from .version import read_workspace_version
from .windows_helpers import resolve_windows_sandbox_helpers


SCRIPT_DIRECTORY = Path(__file__).resolve().parents[1]
REPOSITORY_ROOT = SCRIPT_DIRECTORY.parents[1]
DEFAULT_LOCK = REPOSITORY_ROOT / "third_party" / "ripgrep" / "runtime-lock.json"
DEFAULT_CACHE = REPOSITORY_ROOT / "third_party" / ".cache" / "ripgrep"
DEFAULT_NODE_LOCK = REPOSITORY_ROOT / "third_party" / "node" / "runtime-lock.json"
DEFAULT_NODE_CACHE = REPOSITORY_ROOT / "third_party" / ".cache" / "node"


def generate_protocol_metadata(repository_root: Path, cargo: str) -> Dict[str, object]:
    with tempfile.TemporaryDirectory(prefix=".zeta-protocol-") as temporary:
        output_directory = Path(temporary)
        subprocess.run(
            [
                cargo,
                "run",
                "--quiet",
                "--manifest-path",
                str(repository_root / "Cargo.toml"),
                "-p",
                "zeta-app-server-protocol",
                "--bin",
                "generate_protocol",
                "--",
                "typescript",
                "--out",
                str(output_directory),
            ],
            cwd=repository_root,
            check=True,
        )
        return load_protocol_metadata(
            repository_root,
            output_directory / "types.ts",
        )


def parse_arguments(arguments: Optional[Sequence[str]] = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description=(
            "Build the canonical Zeta package directory with pinned, "
            "checksum-verified runtime executables."
        ),
        formatter_class=argparse.ArgumentDefaultsHelpFormatter,
    )
    parser.add_argument(
        "--target",
        choices=sorted(TARGETS),
        default=None,
        help="Rust target triple. Defaults to the current host target.",
    )
    parser.add_argument(
        "--package-dir",
        type=Path,
        required=True,
        help="New directory to create as the package root.",
    )
    parser.add_argument(
        "--server-bin",
        type=Path,
        help="Prebuilt product-neutral Zeta server executable. If omitted, Cargo builds it.",
    )
    parser.add_argument(
        "--app-server-daemon-bin",
        type=Path,
        help="Prebuilt profile-scoped App Server daemon executable. If omitted, Cargo builds it.",
    )
    parser.add_argument(
        "--code-mode-host-bin",
        type=Path,
        help="Prebuilt isolated Code Mode Host executable. If omitted, Cargo builds it.",
    )
    parser.add_argument(
        "--rg-bin",
        type=Path,
        help="Local ripgrep executable override instead of the locked download.",
    )
    parser.add_argument(
        "--node-bin",
        type=Path,
        help=(
            "Managed Node.js executable override instead of the locked download. "
            "Required for musl targets."
        ),
    )
    parser.add_argument(
        "--javascript-runtime",
        choices=("packaged-node", "host-provided-node"),
        default="packaged-node",
        help=(
            "Package the locked standalone Node runtime, or require a product host "
            "such as Electron to inject an exact Node-compatible executable."
        ),
    )
    parser.add_argument(
        "--bwrap-bin",
        type=Path,
        help=(
            "Prebuilt Linux Bubblewrap executable. If omitted for Linux, "
            "the vendored upstream source is built with Cargo."
        ),
    )
    parser.add_argument(
        "--windows-command-runner-bin",
        type=Path,
        help="Prebuilt Windows AppContainer command runner.",
    )
    parser.add_argument(
        "--windows-sandbox-setup-bin",
        type=Path,
        help="Prebuilt Windows AppContainer profile and ACL setup helper.",
    )
    parser.add_argument(
        "--cargo",
        default="cargo",
        help="Cargo executable used to build first-party package binaries.",
    )
    parser.add_argument(
        "--cargo-profile",
        default="release",
        help="Cargo profile used to build first-party package binaries.",
    )
    parser.add_argument(
        "--ripgrep-lock",
        type=Path,
        default=DEFAULT_LOCK,
        help="Pinned ripgrep runtime lock.",
    )
    parser.add_argument(
        "--cache-root",
        type=Path,
        default=DEFAULT_CACHE,
        help="Verified ripgrep download and extraction cache.",
    )
    parser.add_argument(
        "--node-lock",
        type=Path,
        default=DEFAULT_NODE_LOCK,
        help="Pinned Node.js runtime lock.",
    )
    parser.add_argument(
        "--node-cache-root",
        type=Path,
        default=DEFAULT_NODE_CACHE,
        help="Verified Node.js download and extraction cache.",
    )
    return parser.parse_args(arguments)


def main(arguments: Optional[Sequence[str]] = None) -> int:
    args = parse_arguments(arguments)
    target = args.target or default_target()
    spec = TARGETS[target]
    protocol_metadata = generate_protocol_metadata(REPOSITORY_ROOT, args.cargo)
    server_binary = resolve_server_binary(
        REPOSITORY_ROOT,
        spec,
        args.server_bin,
        cargo=args.cargo,
        cargo_profile=args.cargo_profile,
    )
    app_server_daemon_binary = resolve_app_server_daemon_binary(
        REPOSITORY_ROOT,
        spec,
        args.app_server_daemon_bin,
        cargo=args.cargo,
        cargo_profile=args.cargo_profile,
    )
    code_mode_host_binary = resolve_code_mode_host_binary(
        REPOSITORY_ROOT,
        spec,
        args.code_mode_host_bin,
        cargo=args.cargo,
        cargo_profile=args.cargo_profile,
    )
    ripgrep = resolve_ripgrep(
        spec,
        args.ripgrep_lock.expanduser().resolve(),
        args.cache_root.expanduser().resolve(),
        explicit_binary=args.rg_bin,
    )
    if args.javascript_runtime == "host-provided-node" and args.node_bin is not None:
        raise RuntimeError(
            "--node-bin cannot be used with --javascript-runtime host-provided-node"
        )
    node = (
        resolve_node(
            spec,
            args.node_lock.expanduser().resolve(),
            args.node_cache_root.expanduser().resolve(),
            explicit_binary=args.node_bin,
        )
        if args.javascript_runtime == "packaged-node"
        else None
    )
    bubblewrap = resolve_bubblewrap(
        REPOSITORY_ROOT,
        spec,
        explicit_binary=args.bwrap_bin,
        cargo=args.cargo,
        cargo_profile=args.cargo_profile,
    )
    windows_helpers = resolve_windows_sandbox_helpers(
        REPOSITORY_ROOT,
        spec,
        args.windows_command_runner_bin,
        args.windows_sandbox_setup_bin,
        cargo=args.cargo,
        cargo_profile=args.cargo_profile,
    )
    version = read_workspace_version(REPOSITORY_ROOT / "Cargo.toml")
    output = args.package_dir.expanduser().resolve()
    build_package_directory(
        output,
        REPOSITORY_ROOT,
        version,
        spec,
        server_binary,
        app_server_daemon_binary,
        code_mode_host_binary,
        ripgrep,
        node,
        bubblewrap,
        windows_helpers,
        protocol_metadata=protocol_metadata,
        build_profile=args.cargo_profile,
    )
    print("Built Zeta {} package at {}".format(target, output))
    return 0
