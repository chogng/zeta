"""Command-line interface for building a canonical Zeta package directory."""

import argparse
from pathlib import Path
from typing import Optional, Sequence

from .bubblewrap import resolve_bubblewrap
from .cargo import resolve_zeta_binary
from .layout import build_package_directory
from .ripgrep import resolve_ripgrep
from .targets import TARGETS, default_target
from .version import read_workspace_version
from .windows_helpers import resolve_windows_sandbox_helpers


SCRIPT_DIRECTORY = Path(__file__).resolve().parents[1]
REPOSITORY_ROOT = SCRIPT_DIRECTORY.parent
DEFAULT_LOCK = REPOSITORY_ROOT / "third_party" / "ripgrep" / "runtime-lock.json"
DEFAULT_CACHE = REPOSITORY_ROOT / "third_party" / ".cache" / "ripgrep"
DEFAULT_BUBBLEWRAP_LOCK = (
    REPOSITORY_ROOT / "third_party" / "bubblewrap" / "runtime-lock.json"
)
DEFAULT_BUBBLEWRAP_CACHE = (
    REPOSITORY_ROOT / "third_party" / ".cache" / "bubblewrap"
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
        "--zeta-bin",
        type=Path,
        help="Prebuilt Zeta executable. If omitted, Cargo builds it.",
    )
    parser.add_argument(
        "--rg-bin",
        type=Path,
        help="Local ripgrep executable override instead of the locked download.",
    )
    parser.add_argument(
        "--bwrap-bin",
        type=Path,
        help=(
            "Prebuilt Linux Bubblewrap executable. If omitted for Linux, "
            "the locked upstream source is built with Cargo."
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
        "--bubblewrap-lock",
        type=Path,
        default=DEFAULT_BUBBLEWRAP_LOCK,
        help="Pinned Bubblewrap source runtime lock.",
    )
    parser.add_argument(
        "--bubblewrap-cache-root",
        type=Path,
        default=DEFAULT_BUBBLEWRAP_CACHE,
        help="Verified Bubblewrap source cache.",
    )
    return parser.parse_args(arguments)


def main(arguments: Optional[Sequence[str]] = None) -> int:
    args = parse_arguments(arguments)
    target = args.target or default_target()
    spec = TARGETS[target]
    zeta_binary = resolve_zeta_binary(
        REPOSITORY_ROOT,
        spec,
        args.zeta_bin,
        cargo=args.cargo,
        cargo_profile=args.cargo_profile,
    )
    ripgrep = resolve_ripgrep(
        spec,
        args.ripgrep_lock.expanduser().resolve(),
        args.cache_root.expanduser().resolve(),
        explicit_binary=args.rg_bin,
    )
    bubblewrap = resolve_bubblewrap(
        REPOSITORY_ROOT,
        spec,
        args.bubblewrap_lock.expanduser().resolve(),
        args.bubblewrap_cache_root.expanduser().resolve(),
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
    version = read_workspace_version(REPOSITORY_ROOT / "zeta-rs" / "Cargo.toml")
    output = args.package_dir.expanduser().resolve()
    build_package_directory(
        output,
        REPOSITORY_ROOT,
        version,
        spec,
        zeta_binary,
        ripgrep,
        bubblewrap,
        windows_helpers,
    )
    print("Built Zeta {} package at {}".format(target, output))
    return 0
