"""Shared signing and verification logic for staged zeterm packages."""

from __future__ import annotations

import hashlib
import json
import os
import subprocess
import sys
from pathlib import Path
from typing import Callable, Dict, List, Optional, Sequence


CommandRunner = Callable[[Sequence[str]], None]
SUPPORTED_PLATFORMS = {"darwin", "linux", "windows"}


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as source:
        for block in iter(lambda: source.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def host_platform() -> str:
    if sys.platform == "darwin":
        return "darwin"
    if sys.platform.startswith("linux"):
        return "linux"
    if sys.platform.startswith("win"):
        return "windows"
    raise RuntimeError(f"unsupported signing host platform: {sys.platform}")


def load_json(path: Path) -> Dict[str, object]:
    try:
        value = json.loads(path.read_text())
    except (OSError, json.JSONDecodeError) as error:
        raise RuntimeError(f"could not read JSON file {path}: {error}") from error
    if not isinstance(value, dict):
        raise RuntimeError(f"expected a JSON object in {path}")
    return value


def write_json(path: Path, value: Dict[str, object]) -> None:
    temporary = path.with_name(f".{path.name}.tmp")
    temporary.write_text(json.dumps(value, indent=2) + "\n")
    os.replace(temporary, path)


def package_context(package_dir: Path) -> Dict[str, object]:
    package_dir = package_dir.expanduser().resolve()
    metadata_path = package_dir / "zeterm-package.json"
    metadata = load_json(metadata_path)
    signing = metadata.get("signing")
    binary = metadata.get("binary")
    if not isinstance(signing, dict) or not isinstance(binary, dict):
        raise RuntimeError(f"invalid zeterm package metadata: {metadata_path}")

    binary_path = binary.get("path")
    policy_name = signing.get("policy")
    record_name = signing.get("signatureRecord", "zeterm-signature.json")
    if not isinstance(binary_path, str) or not isinstance(policy_name, str):
        raise RuntimeError(f"invalid binary or policy path in {metadata_path}")
    if not isinstance(record_name, str):
        raise RuntimeError(f"invalid signature record path in {metadata_path}")

    artifact = (package_dir / binary_path).resolve()
    policy_path = (package_dir / policy_name).resolve()
    record_path = (package_dir / record_name).resolve()
    for path, label in ((artifact, "binary"), (policy_path, "policy"), (record_path, "signature record")):
        try:
            path.relative_to(package_dir)
        except ValueError as error:
            raise RuntimeError(f"{label} path escapes package directory: {path}") from error

    if not artifact.is_file():
        raise RuntimeError(f"zeterm executable does not exist: {artifact}")
    if not policy_path.is_file():
        raise RuntimeError(f"signing policy does not exist: {policy_path}")

    policy = load_json(policy_path)
    platforms = policy.get("platforms")
    if not isinstance(platforms, dict):
        raise RuntimeError(f"signing policy has no platform map: {policy_path}")

    return {
        "package_dir": package_dir,
        "metadata_path": metadata_path,
        "metadata": metadata,
        "binary": binary,
        "artifact": artifact,
        "policy": policy,
        "platforms": platforms,
        "record_path": record_path,
    }


def platform_config(context: Dict[str, object], platform: str) -> Dict[str, object]:
    if platform not in SUPPORTED_PLATFORMS:
        raise RuntimeError(f"unsupported zeterm signing platform: {platform}")
    platforms = context["platforms"]
    assert isinstance(platforms, dict)
    config = platforms.get(platform)
    if not isinstance(config, dict):
        raise RuntimeError(f"signing policy has no {platform} configuration")
    tool = config.get("tool")
    identity_environment = config.get("identityEnvironment")
    signature_mode = config.get("signatureMode")
    if not isinstance(tool, str) or not isinstance(identity_environment, str):
        raise RuntimeError(f"incomplete {platform} signing policy")
    if signature_mode not in {"embedded", "detached"}:
        raise RuntimeError(f"invalid {platform} signature mode: {signature_mode}")
    if signature_mode == "detached" and not isinstance(config.get("signatureFile"), str):
        raise RuntimeError(f"detached {platform} signing requires signatureFile")
    return config


def signature_path(context: Dict[str, object], config: Dict[str, object]) -> Optional[Path]:
    if config.get("signatureMode") != "detached":
        return None
    package_dir = context["package_dir"]
    assert isinstance(package_dir, Path)
    value = config.get("signatureFile")
    assert isinstance(value, str)
    path = (package_dir / value).resolve()
    try:
        path.relative_to(package_dir)
    except ValueError as error:
        raise RuntimeError(f"signature path escapes package directory: {path}") from error
    return path


def identity_for(config: Dict[str, object]) -> str:
    environment = config["identityEnvironment"]
    assert isinstance(environment, str)
    identity = os.environ.get(environment)
    if not identity:
        raise RuntimeError(f"release signing requires environment variable {environment}")
    return identity


def run_command(command: Sequence[str], runner: Optional[CommandRunner]) -> None:
    try:
        if runner is not None:
            runner(command)
        else:
            subprocess.run(list(command), check=True)
    except FileNotFoundError as error:
        raise RuntimeError(f"signing tool is not installed: {command[0]}") from error
    except subprocess.CalledProcessError as error:
        rendered = " ".join(command)
        raise RuntimeError(f"signing command failed with exit code {error.returncode}: {rendered}") from error


def sign_command(
    context: Dict[str, object],
    config: Dict[str, object],
    platform: str,
    identity: str,
) -> List[str]:
    tool = config["tool"]
    assert isinstance(tool, str)
    artifact = context["artifact"]
    assert isinstance(artifact, Path)
    if platform == "darwin":
        return [tool, "--force", "--sign", identity, "--timestamp", "--options", "runtime", str(artifact)]
    if platform == "linux":
        signature = signature_path(context, config)
        assert signature is not None
        return [tool, "sign-blob", "--yes", "--key", identity, "--output-signature", str(signature), str(artifact)]

    command = [tool, "sign", "/fd", "SHA256", "/f", identity]
    command.append(str(artifact))
    return command


def verify_command(
    context: Dict[str, object],
    config: Dict[str, object],
    platform: str,
) -> List[str]:
    tool = config["tool"]
    assert isinstance(tool, str)
    artifact = context["artifact"]
    assert isinstance(artifact, Path)
    if platform == "darwin":
        return [tool, "--verify", "--strict", str(artifact)]
    if platform == "linux":
        signature = signature_path(context, config)
        assert signature is not None
        return [tool, "verify-blob", "--key", identity_for(config), "--signature", str(signature), str(artifact)]
    return [tool, "verify", "/pa", str(artifact)]


def _metadata_digest(context: Dict[str, object]) -> str:
    binary = context["binary"]
    assert isinstance(binary, dict)
    value = binary.get("sha256")
    if not isinstance(value, str) or len(value) != 64:
        raise RuntimeError("zeterm package metadata has no valid binary sha256")
    return value


def sign_package(
    package_dir: Path,
    platform: str,
    runner: Optional[CommandRunner] = None,
) -> Dict[str, object]:
    context = package_context(package_dir)
    config = platform_config(context, platform)
    metadata = context["metadata"]
    assert isinstance(metadata, dict)
    signing = metadata["signing"]
    assert isinstance(signing, dict)
    if signing.get("status") != "unsigned":
        raise RuntimeError(f"package is not unsigned; refusing to sign status={signing.get('status')}")

    artifact = context["artifact"]
    assert isinstance(artifact, Path)
    unsigned_digest = _metadata_digest(context)
    actual_digest = sha256(artifact)
    if actual_digest != unsigned_digest:
        raise RuntimeError("staged binary digest does not match zeterm-package.json")

    detached_signature = signature_path(context, config)
    if detached_signature is not None and detached_signature.exists():
        raise RuntimeError(f"refusing to replace existing signature: {detached_signature}")

    run_command(sign_command(context, config, platform, identity_for(config)), runner)
    if detached_signature is not None and not detached_signature.is_file():
        raise RuntimeError(f"signing tool did not create signature: {detached_signature}")

    signed_digest = sha256(artifact)
    record = {
        "formatVersion": 1,
        "product": "zeterm",
        "platform": platform,
        "tool": config["tool"],
        "artifact": metadata["binary"]["path"],
        "unsignedSha256": unsigned_digest,
        "signedSha256": signed_digest,
        "status": "signed",
    }
    if detached_signature is not None:
        record["signatureFile"] = str(detached_signature.relative_to(context["package_dir"]))

    binary = metadata["binary"]
    assert isinstance(binary, dict)
    binary["sha256"] = signed_digest
    signing["status"] = "signed"
    signing["unsignedSha256"] = unsigned_digest
    signing["signedSha256"] = signed_digest
    signing["signatureRecord"] = str(context["record_path"].relative_to(context["package_dir"]))
    write_json(context["record_path"], record)
    write_json(context["metadata_path"], metadata)
    return record


def verify_package(
    package_dir: Path,
    platform: str,
    runner: Optional[CommandRunner] = None,
) -> Dict[str, object]:
    context = package_context(package_dir)
    config = platform_config(context, platform)
    metadata = context["metadata"]
    assert isinstance(metadata, dict)
    signing = metadata["signing"]
    assert isinstance(signing, dict)
    if signing.get("status") not in {"signed", "verified"}:
        raise RuntimeError("package must be signed before verification")

    artifact = context["artifact"]
    assert isinstance(artifact, Path)
    digest = sha256(artifact)
    if digest != _metadata_digest(context):
        raise RuntimeError("signed binary digest does not match zeterm-package.json")
    record_path = context["record_path"]
    if not record_path.is_file():
        raise RuntimeError(f"signature record does not exist: {record_path}")
    record = load_json(record_path)
    if record.get("platform") != platform or record.get("status") not in {"signed", "verified"}:
        raise RuntimeError("signature record does not match the requested platform or state")
    if record.get("signedSha256") != digest:
        raise RuntimeError("signature record digest does not match the staged binary")

    detached_signature = signature_path(context, config)
    if detached_signature is not None and not detached_signature.is_file():
        raise RuntimeError(f"signature file does not exist: {detached_signature}")

    run_command(verify_command(context, config, platform), runner)
    record["status"] = "verified"
    record["verifiedSha256"] = digest
    signing["status"] = "verified"
    signing["verifiedSha256"] = digest
    write_json(record_path, record)
    write_json(context["metadata_path"], metadata)
    return record
