#!/usr/bin/env python3
"""Build, sign, and verify one app package on any supported host."""

from __future__ import annotations

import os
import sys
from pathlib import Path

SCRIPT_DIR = Path(__file__).resolve().parent
REPO_ROOT = SCRIPT_DIR.parents[1]
BUILD_LIB_DIR = REPO_ROOT / "build" / "lib"
if str(BUILD_LIB_DIR) not in sys.path:
    sys.path.insert(0, str(BUILD_LIB_DIR))

from app_signing import sign_package, verify_package  # noqa: E402
from build_app_package import (  # noqa: E402
    build_package,
    host_target,
    remote_runtime_network_release,
    resolve_binary,
)
from remote_runtime_bundle import validate_remote_runtime_bundle  # noqa: E402


def env_path(name: str, *, required: bool = False) -> Path | None:
    value = os.environ.get(name)
    if value:
        return Path(value).resolve()
    if required:
        raise RuntimeError(f"{name} is required")
    return None


def main() -> int:
    package_dir = env_path("APP_PACKAGE_DIR", required=True)
    assert package_dir is not None

    target = os.environ.get("APP_TARGET") or host_target("cargo")
    profile = os.environ.get("APP_CARGO_PROFILE", "release")
    remote_runtime_path = env_path("APP_REMOTE_RUNTIME_BUNDLE")
    remote_runtime_bundle = (
        validate_remote_runtime_bundle(remote_runtime_path)
        if remote_runtime_path is not None
        else None
    )
    remote_runtime_url = os.environ.get("APP_REMOTE_RUNTIME_CATALOG_URL")
    remote_runtime_sha256 = os.environ.get("APP_REMOTE_RUNTIME_CATALOG_SHA256")
    if (remote_runtime_url is None) != (remote_runtime_sha256 is None):
        raise RuntimeError(
            "APP_REMOTE_RUNTIME_CATALOG_URL and "
            "APP_REMOTE_RUNTIME_CATALOG_SHA256 are required together"
        )
    remote_runtime_release = (
        remote_runtime_network_release(remote_runtime_url, remote_runtime_sha256)
        if remote_runtime_url is not None and remote_runtime_sha256 is not None
        else None
    )
    binary = resolve_binary(
        "cargo",
        profile,
        target,
        env_path("APP_BINARY"),
        remote_runtime_bundle,
        remote_runtime_release,
    )
    if not binary.is_file():
        raise RuntimeError(f"app executable does not exist: {binary}")

    build_package(
        package_dir,
        binary,
        target,
        profile,
        remote_runtime_bundle,
        remote_runtime_release,
    )
    sign_package(package_dir)
    verify_package(package_dir)
    print(f"Verified release package at {package_dir}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
