"""Build or validate the pinned Bubblewrap helper for Linux packages."""

import hashlib
import json
import os
import shutil
import subprocess
import tarfile
import tempfile
from dataclasses import dataclass
from pathlib import Path
from pathlib import PurePosixPath
from typing import Any, Dict, List, Optional
from urllib.request import Request, urlopen

from .cargo import validate_input_binary
from .targets import TargetSpec


DOWNLOAD_TIMEOUT_SECONDS = 60
SOURCE_DIGEST_MARKER = ".zeta-source-sha256"


@dataclass(frozen=True)
class BubblewrapResolution:
    executable: Path
    version: str
    binary_sha256: str
    source_archive: str
    source_archive_sha256: str
    license_files: List[Path]
    source: str


@dataclass(frozen=True)
class SourceLock:
    version: str
    archive_name: str
    archive_size: int
    archive_sha256: str
    archive_url: str
    archive_root: str
    members: List[str]


def resolve_bubblewrap(
    repository_root: Path,
    spec: TargetSpec,
    lock_path: Path,
    cache_root: Path,
    explicit_binary: Optional[Path],
    cargo: str,
    cargo_profile: str,
) -> Optional[BubblewrapResolution]:
    if not spec.is_linux:
        if explicit_binary is not None:
            raise RuntimeError("--bwrap-bin is only supported for Linux packages")
        return None

    source_lock = load_source_lock(lock_path)
    source_directory = materialize_source(source_lock, cache_root)
    if explicit_binary is not None:
        executable = validate_input_binary(
            explicit_binary, "Bubblewrap executable", "--bwrap-bin", False
        )
        source = "local-override"
    else:
        executable = build_bubblewrap(
            repository_root,
            spec,
            source_directory,
            cargo,
            cargo_profile,
        )
        source = "source-build"
    return BubblewrapResolution(
        executable=executable,
        version=source_lock.version,
        binary_sha256=sha256(executable),
        source_archive=source_lock.archive_name,
        source_archive_sha256=source_lock.archive_sha256,
        license_files=[
            source_directory / "COPYING",
        ],
        source=source,
    )


def load_source_lock(lock_path: Path) -> SourceLock:
    try:
        value = json.loads(lock_path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        raise RuntimeError(
            "Could not read Bubblewrap lock {}: {}".format(lock_path, error)
        ) from error
    if value.get("schemaVersion") != 1 or value.get("runtime") != "bubblewrap-source":
        raise RuntimeError("Unsupported Bubblewrap lock schema in {}".format(lock_path))

    source = required_mapping(value, "source")
    archive = required_mapping(value, "archive")
    repository = required_string(source, "repository").rstrip("/")
    release = required_string(source, "release")
    version = required_path_component(value, "version")
    archive_name = required_path_component(archive, "name")
    archive_format = required_string(archive, "format")
    if archive_format != "tar.xz":
        raise RuntimeError(
            "Unsupported Bubblewrap source format {!r}; expected tar.xz".format(
                archive_format
            )
        )
    digest = required_string(archive, "sha256")
    if len(digest) != 64 or any(character not in "0123456789abcdef" for character in digest):
        raise RuntimeError("Invalid Bubblewrap source SHA-256")
    members = archive.get("members")
    if (
        not isinstance(members, list)
        or not members
        or any(not isinstance(member, str) or not member for member in members)
    ):
        raise RuntimeError("Bubblewrap lock members must be non-empty strings")
    archive_root = required_archive_path(archive, "root")
    members = [
        validate_archive_path(member, "Bubblewrap lock archive member")
        for member in members
    ]
    for required_member in ("COPYING", "bubblewrap.c"):
        if required_member not in members:
            raise RuntimeError(
                "Bubblewrap lock does not include required member {!r}".format(
                    required_member
                )
            )
    url = archive.get("url")
    if not isinstance(url, str):
        url = "{}/releases/download/{}/{}".format(repository, release, archive_name)
    return SourceLock(
        version=version,
        archive_name=archive_name,
        archive_size=required_positive_integer(archive, "size"),
        archive_sha256=digest,
        archive_url=url,
        archive_root=archive_root,
        members=list(members),
    )


def materialize_source(source_lock: SourceLock, cache_root: Path) -> Path:
    version_directory = cache_root / source_lock.version
    archive_path = version_directory / source_lock.archive_name
    if not archive_is_valid(archive_path, source_lock):
        archive_path.unlink(missing_ok=True)
        download_and_verify(source_lock, archive_path)

    source_directory = version_directory / "source"
    if source_is_valid(source_directory, source_lock):
        return source_directory
    staging = Path(
        tempfile.mkdtemp(prefix=".source.partial-", dir=str(version_directory))
    )
    try:
        with tarfile.open(str(archive_path), "r:xz") as archive:
            for relative_member in source_lock.members:
                archive_member = "{}/{}".format(
                    source_lock.archive_root, relative_member
                )
                try:
                    member = archive.getmember(archive_member)
                except KeyError as error:
                    raise RuntimeError(
                        "Bubblewrap archive {} is missing {!r}".format(
                            archive_path, archive_member
                        )
                    ) from error
                if not member.isfile():
                    raise RuntimeError(
                        "Bubblewrap archive member {!r} is not a regular file".format(
                            archive_member
                        )
                    )
                extracted = archive.extractfile(member)
                if extracted is None:
                    raise RuntimeError(
                        "Could not read Bubblewrap source member {!r}".format(
                            archive_member
                        )
                    )
                destination = staging / relative_member
                destination.parent.mkdir(parents=True, exist_ok=True)
                with extracted, open(destination, "wb") as output:
                    shutil.copyfileobj(extracted, output)
        (staging / SOURCE_DIGEST_MARKER).write_text(
            source_lock.archive_sha256 + "\n", encoding="utf-8"
        )
        if source_directory.exists():
            shutil.rmtree(source_directory)
        staging.rename(source_directory)
    except Exception:
        shutil.rmtree(staging, ignore_errors=True)
        raise
    return source_directory


def source_is_valid(source_directory: Path, source_lock: SourceLock) -> bool:
    marker = source_directory / SOURCE_DIGEST_MARKER
    try:
        if marker.read_text(encoding="utf-8").strip() != source_lock.archive_sha256:
            return False
    except OSError:
        return False
    return all((source_directory / member).is_file() for member in source_lock.members)


def build_bubblewrap(
    repository_root: Path,
    spec: TargetSpec,
    source_directory: Path,
    cargo: str,
    cargo_profile: str,
) -> Path:
    rust_workspace = repository_root
    target_directory = repository_root / "target"
    environment = dict(os.environ)
    environment["ZETA_BWRAP_SOURCE_DIR"] = str(source_directory)
    command = [
        cargo,
        "build",
        "--manifest-path",
        str(repository_root / "Cargo.toml"),
        "--package",
        "zeta-bwrap",
        "--bin",
        "bwrap",
        "--profile",
        cargo_profile,
        "--target",
        spec.target,
        "--target-dir",
        str(target_directory),
    ]
    subprocess.run(command, check=True, env=environment)
    profile_directory = "debug" if cargo_profile == "dev" else cargo_profile
    executable = target_directory / spec.target / profile_directory / "bwrap"
    return validate_input_binary(
        executable, "built Bubblewrap executable", cargo, False
    )


def archive_is_valid(path: Path, source_lock: SourceLock) -> bool:
    if not path.is_file():
        return False
    return (
        path.stat().st_size == source_lock.archive_size
        and sha256(path) == source_lock.archive_sha256
    )


def download_and_verify(source_lock: SourceLock, destination: Path) -> None:
    destination.parent.mkdir(parents=True, exist_ok=True)
    temporary = destination.with_name(destination.name + ".partial")
    temporary.unlink(missing_ok=True)
    request = Request(
        source_lock.archive_url, headers={"User-Agent": "zeta-package-builder"}
    )
    try:
        with urlopen(request, timeout=DOWNLOAD_TIMEOUT_SECONDS) as response:
            with open(temporary, "wb") as output:
                shutil.copyfileobj(response, output)
        if temporary.stat().st_size != source_lock.archive_size:
            raise RuntimeError(
                "Bubblewrap source archive has size {}, expected {}".format(
                    temporary.stat().st_size, source_lock.archive_size
                )
            )
        actual_digest = sha256(temporary)
        if actual_digest != source_lock.archive_sha256:
            raise RuntimeError(
                "Bubblewrap source archive has SHA-256 {}, expected {}".format(
                    actual_digest, source_lock.archive_sha256
                )
            )
        os.replace(str(temporary), str(destination))
    except Exception:
        temporary.unlink(missing_ok=True)
        destination.unlink(missing_ok=True)
        raise


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with open(path, "rb") as contents:
        for chunk in iter(lambda: contents.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def required_mapping(value: Dict[str, Any], key: str) -> Dict[str, Any]:
    result = value.get(key)
    if not isinstance(result, dict):
        raise RuntimeError("Bubblewrap lock field {!r} must be an object".format(key))
    return result


def required_string(value: Dict[str, Any], key: str) -> str:
    result = value.get(key)
    if not isinstance(result, str):
        raise RuntimeError("Bubblewrap lock field {!r} must be a string".format(key))
    return result


def required_positive_integer(value: Dict[str, Any], key: str) -> int:
    result = value.get(key)
    if isinstance(result, bool) or not isinstance(result, int) or result <= 0:
        raise RuntimeError(
            "Bubblewrap lock field {!r} must be a positive integer".format(key)
        )
    return result


def required_path_component(value: Dict[str, Any], key: str) -> str:
    result = required_string(value, key)
    if result in (".", "..") or PurePosixPath(result).name != result:
        raise RuntimeError(
            "Bubblewrap lock field {!r} must be one path component".format(key)
        )
    return result


def required_archive_path(value: Dict[str, Any], key: str) -> str:
    return validate_archive_path(
        required_string(value, key),
        "Bubblewrap lock field {!r}".format(key),
    )


def validate_archive_path(path: str, description: str) -> str:
    parsed = PurePosixPath(path)
    if parsed.is_absolute() or any(part in ("", ".", "..") for part in parsed.parts):
        raise RuntimeError("{} must be a safe relative path".format(description))
    return path
