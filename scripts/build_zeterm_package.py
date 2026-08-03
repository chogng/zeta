"""Build a deterministic unsigned zeterm package directory."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import shutil
import subprocess
import tempfile
from pathlib import Path
from typing import Optional


REPOSITORY_ROOT = Path(__file__).resolve().parents[1]
ZETERM_ROOT = REPOSITORY_ROOT / "zeterm"


def host_target(cargo: str) -> str:
    result = subprocess.run(
        [cargo, "-vV"],
        check=True,
        capture_output=True,
        text=True,
    )
    for line in result.stdout.splitlines():
        if line.startswith("host:"):
            return line.split(":", 1)[1].strip()
    raise RuntimeError("cargo -vV did not report a host target")


def package_version() -> str:
    manifest = (ZETERM_ROOT / "Cargo.toml").read_text()
    package_section = manifest.split("[package]", 1)[1].split("[", 1)[0]
    for line in package_section.splitlines():
        if line.strip().startswith("version"):
            return line.split('"', 2)[1]
    raise RuntimeError("zeterm/Cargo.toml does not define a package version")


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as source:
        for block in iter(lambda: source.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def resolve_binary(cargo: str, profile: str, target: Optional[str], explicit: Optional[Path]) -> Path:
    if explicit is not None:
        return explicit.expanduser().resolve()

    command = [
        cargo,
        "build",
        "--manifest-path",
        str(ZETERM_ROOT / "Cargo.toml"),
        "--package",
        "zeterm",
        "--bin",
        "zeterm",
        "--locked",
        "--profile",
        profile,
    ]
    if target:
        command.extend(["--target", target])
    subprocess.run(command, check=True)

    profile_directory = "debug" if profile == "dev" else profile
    binary = REPOSITORY_ROOT / "target"
    if target:
        binary /= target
    binary /= profile_directory
    return binary / ("zeterm.exe" if target and "windows" in target else "zeterm")


def build_package(output: Path, binary: Path, target: str, profile: str) -> None:
    if output.exists():
        raise RuntimeError(f"refusing to replace existing package directory: {output}")
    output.parent.mkdir(parents=True, exist_ok=True)
    staging = Path(tempfile.mkdtemp(prefix=f".{output.name}.", dir=output.parent))
    try:
        staged_binary = staging / "bin" / binary.name
        staged_binary.parent.mkdir(parents=True)
        shutil.copy2(binary, staged_binary)
        if os.name != "nt":
            staged_binary.chmod(staged_binary.stat().st_mode | 0o111)
        shutil.copy2(
            ZETERM_ROOT / "packaging" / "zeterm-signing-policy.json",
            staging / "zeterm-signing-policy.json",
        )
        metadata = {
            "formatVersion": 1,
            "product": "zeterm",
            "version": package_version(),
            "target": target,
            "profile": profile,
            "binary": {
                "path": f"bin/{binary.name}",
                "sha256": sha256(staged_binary),
            },
            "signing": {
                "status": "unsigned",
                "requiredForRelease": True,
                "policy": "zeterm-signing-policy.json",
            },
        }
        (staging / "zeterm-package.json").write_text(
            json.dumps(metadata, indent=2) + "\n"
        )
        staging.rename(output)
    except BaseException:
        shutil.rmtree(staging, ignore_errors=True)
        raise


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--package-dir", type=Path, required=True)
    parser.add_argument("--target")
    parser.add_argument("--zeterm-bin", type=Path)
    parser.add_argument("--cargo", default="cargo")
    parser.add_argument("--cargo-profile", default="release")
    args = parser.parse_args()

    target = args.target or host_target(args.cargo)
    binary = resolve_binary(args.cargo, args.cargo_profile, args.target, args.zeterm_bin)
    binary = binary.expanduser().resolve()
    if not binary.is_file():
        raise RuntimeError(f"zeterm executable does not exist: {binary}")
    output = args.package_dir.expanduser().resolve()
    build_package(output, binary, target, args.cargo_profile)
    print(f"Built zeterm {target} package at {output}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
