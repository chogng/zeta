"""Build the current Zeta package generation and launch the Code TUI against it."""

from __future__ import annotations

import json
import os
import platform
import re
import subprocess
import sys
from pathlib import Path


REPOSITORY_ROOT = Path(__file__).resolve().parents[1]
MANIFEST_PATTERN = re.compile(r"\d{20}\.json")
PACKAGE_DIRECTORY_PATTERN = re.compile(r"packages/[0-9A-Za-z][0-9A-Za-z.+-]*/[a-f0-9]{64}")


def development_root() -> Path:
    targets = {
        ("darwin", "arm64"): "aarch64-apple-darwin",
        ("darwin", "x86_64"): "x86_64-apple-darwin",
        ("linux", "aarch64"): "aarch64-unknown-linux-gnu",
        ("linux", "x86_64"): "x86_64-unknown-linux-gnu",
        ("windows", "arm64"): "aarch64-pc-windows-msvc",
        ("windows", "amd64"): "x86_64-pc-windows-msvc",
    }
    host = (platform.system().lower(), platform.machine().lower())
    target = targets.get(host)
    if target is None:
        raise RuntimeError(f"unsupported Zeta development host: {host[0]}/{host[1]}")
    return REPOSITORY_ROOT / ".build" / "zeta-package" / "dev" / "store-v1" / target / "host-provided-node" / "dev-small"


def current_package() -> Path:
    root = development_root()
    manifest_directory = root / "manifests"
    manifests = sorted(path for path in manifest_directory.iterdir() if MANIFEST_PATTERN.fullmatch(path.name))
    if not manifests:
        raise RuntimeError(f"Zeta development package has no published manifest: {manifest_directory}")
    manifest_path = manifests[-1]
    manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
    sequence = int(manifest_path.name[:20])
    if (
        not isinstance(manifest, dict)
        or manifest.get("formatVersion") != 1
        or manifest.get("sequence") != sequence
        or not isinstance(manifest.get("directory"), str)
        or PACKAGE_DIRECTORY_PATTERN.fullmatch(manifest["directory"]) is None
    ):
        raise RuntimeError(f"invalid Zeta development package manifest: {manifest_path}")
    package_root = root.joinpath(*manifest["directory"].split("/"))
    if not package_root.is_dir():
        raise RuntimeError(f"Zeta development package is missing: {package_root}")
    return package_root


def main(arguments: list[str] | None = None) -> int:
    prepared = subprocess.run(
        ["node", "build/zeta-package/prepareDevPackage.ts"],
        cwd=REPOSITORY_ROOT,
        check=False,
    )
    if prepared.returncode != 0:
        return prepared.returncode

    package_root = current_package()
    executable_suffix = ".exe" if os.name == "nt" else ""
    daemon = package_root / "bin" / f"zeta-app-server-daemon{executable_suffix}"
    product_services = (
        package_root / "zeta-resources" / "product-services" / "product-services.json"
    )
    if not daemon.is_file():
        raise RuntimeError(f"Zeta App Server daemon is missing: {daemon}")
    if not product_services.is_file():
        raise RuntimeError(f"Zeta product services are missing: {product_services}")

    environment = os.environ.copy()
    environment["ZETA_APP_SERVER_DAEMON_PATH"] = str(daemon.resolve())
    environment["ZETA_PRODUCT_SERVICES_PATH"] = str(product_services.resolve())
    return subprocess.run(
        [
            sys.executable,
            "-B",
            "scripts/cargo.py",
            "run",
            "-p",
            "zeta-cli",
            "--bin",
            "zeta",
            "--",
            *(arguments or []),
        ],
        cwd=REPOSITORY_ROOT,
        env=environment,
        check=False,
    ).returncode


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
