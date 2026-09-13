"""Shared macOS and Windows release signing commands."""

from __future__ import annotations

import hashlib
import os
import subprocess
from dataclasses import dataclass
from pathlib import Path
from typing import Callable, Optional, Sequence


CommandRunner = Callable[[Sequence[str]], None]
WINDOWS_TIMESTAMP_URL = "http://timestamp.digicert.com"


@dataclass(frozen=True)
class SignedArtifact:
    path: Path
    unsigned_sha256: str
    signed_sha256: str


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as source:
        for block in iter(lambda: source.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def require_environment(name: str) -> str:
    value = os.environ.get(name)
    if not value:
        raise RuntimeError(f"release signing requires environment variable {name}")
    return value


def run_command(command: Sequence[str], runner: Optional[CommandRunner] = None) -> None:
    try:
        if runner is not None:
            runner(command)
        else:
            subprocess.run(list(command), check=True)
    except FileNotFoundError as error:
        raise RuntimeError(f"signing tool is not installed: {command[0]}") from error
    except subprocess.CalledProcessError as error:
        rendered = " ".join(command)
        raise RuntimeError(
            f"signing command failed with exit code {error.returncode}: {rendered}"
        ) from error


def sign_command(artifact: Path, platform: str) -> list[str]:
    if platform == "darwin":
        return [
            os.environ.get("ASH_MACOS_CODESIGN") or "codesign",
            "--force",
            "--sign",
            require_environment("ASH_MACOS_SIGNING_IDENTITY"),
            "--timestamp",
            "--options",
            "runtime",
            str(artifact),
        ]
    if platform == "windows":
        return [
            os.environ.get("ASH_WINDOWS_SIGNTOOL") or "signtool",
            "sign",
            "/fd",
            "SHA256",
            "/sha1",
            require_environment("ASH_WINDOWS_SIGNING_THUMBPRINT"),
            "/tr",
            os.environ.get("ASH_WINDOWS_TIMESTAMP_URL") or WINDOWS_TIMESTAMP_URL,
            "/td",
            "SHA256",
            str(artifact),
        ]
    raise RuntimeError(f"system signing is unsupported for platform {platform}")


def verify_command(artifact: Path, platform: str) -> list[str]:
    if platform == "darwin":
        return [
            os.environ.get("ASH_MACOS_CODESIGN") or "codesign",
            "--verify",
            "--strict",
            "--verbose=2",
            str(artifact),
        ]
    if platform == "windows":
        return [
            os.environ.get("ASH_WINDOWS_SIGNTOOL") or "signtool",
            "verify",
            "/pa",
            "/all",
            "/v",
            str(artifact),
        ]
    raise RuntimeError(f"system signing is unsupported for platform {platform}")


def sign_and_verify(
    artifact: Path,
    platform: str,
    runner: Optional[CommandRunner] = None,
) -> SignedArtifact:
    artifact = artifact.expanduser().resolve()
    if artifact.is_symlink() or not artifact.is_file():
        raise RuntimeError(f"release artifact is not a regular file: {artifact}")
    unsigned_sha256 = sha256(artifact)
    run_command(sign_command(artifact, platform), runner)
    run_command(verify_command(artifact, platform), runner)
    return SignedArtifact(artifact, unsigned_sha256, sha256(artifact))


def notarize(
    artifact: Path,
    runner: Optional[CommandRunner] = None,
    *,
    staple: bool = False,
) -> None:
    artifact = artifact.expanduser().resolve()
    if artifact.is_symlink() or not artifact.exists():
        raise RuntimeError(f"notarization artifact does not exist: {artifact}")
    profile = require_environment("ASH_MACOS_NOTARY_PROFILE")
    xcrun = os.environ.get("ASH_MACOS_XCRUN") or "xcrun"
    run_command(
        [
            xcrun,
            "notarytool",
            "submit",
            str(artifact),
            "--keychain-profile",
            profile,
            "--wait",
        ],
        runner,
    )
    if staple:
        run_command([xcrun, "stapler", "staple", str(artifact)], runner)
        run_command([xcrun, "stapler", "validate", str(artifact)], runner)
