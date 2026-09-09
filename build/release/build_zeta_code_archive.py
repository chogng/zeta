#!/usr/bin/env python3
"""Create a deterministic managed Zeta Code archive and SHA-256 sidecar."""

from __future__ import annotations

import argparse
import gzip
import hashlib
import json
import os
import stat
import sys
import tarfile
from pathlib import Path


REPOSITORY_ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPOSITORY_ROOT))

from build.lib.zeta_build.targets import target_spec  # noqa: E402
from build.release.zeta_package.layout import validate_package_directory  # noqa: E402


def create_archive(package: Path, output: Path) -> Path:
    package = package.expanduser().resolve()
    output = output.expanduser().resolve()
    if output.exists() or output.with_suffix(output.suffix + ".sha256").exists():
        raise RuntimeError(f"Refusing to replace an existing release artifact: {output}")
    metadata = json.loads((package / "zeta-package.json").read_text(encoding="utf-8"))
    target = metadata.get("target")
    components = metadata.get("components")
    expected_name = f"zeta-code-{target}.tar.gz"
    if (
        metadata.get("layoutVersion") != 2
        or not isinstance(target, str)
        or not isinstance(components, dict)
        or "cli" not in components
        or output.name != expected_name
    ):
        raise RuntimeError("Zeta Code release package identity is invalid")
    validate_package_directory(package, target_spec(target))
    output.parent.mkdir(parents=True, exist_ok=True)
    temporary = output.with_name(f".{output.name}.{os.getpid()}.part")
    try:
        with temporary.open("xb") as raw:
            with gzip.GzipFile(fileobj=raw, mode="wb", filename="", mtime=0) as compressed:
                with tarfile.open(fileobj=compressed, mode="w") as archive:
                    for path in package_paths(package):
                        relative = path.relative_to(package).as_posix()
                        archive.add(
                            path,
                            arcname=relative,
                            recursive=False,
                            filter=normalized_tar_info,
                        )
        temporary.replace(output)
    except Exception:
        temporary.unlink(missing_ok=True)
        raise
    digest = sha256(output)
    checksum = output.with_suffix(output.suffix + ".sha256")
    checksum.write_text(f"{digest}  {output.name}\n", encoding="ascii")
    return checksum


def package_paths(package: Path) -> list[Path]:
    paths: list[Path] = []
    for root, directories, files in os.walk(package):
        root_path = Path(root)
        directories.sort()
        files.sort()
        for name in directories:
            path = root_path / name
            if path.is_symlink():
                raise RuntimeError(f"Package contains a symbolic path: {path}")
            paths.append(path)
        for name in files:
            path = root_path / name
            if path.is_symlink() or not path.is_file():
                raise RuntimeError(f"Package contains an unsupported path: {path}")
            paths.append(path)
    return sorted(paths, key=lambda path: path.relative_to(package).as_posix())


def normalized_tar_info(info: tarfile.TarInfo) -> tarfile.TarInfo:
    info.uid = 0
    info.gid = 0
    info.uname = ""
    info.gname = ""
    info.mtime = 0
    if info.isdir():
        info.mode = 0o755
    elif info.isfile():
        info.mode = 0o755 if info.mode & stat.S_IXUSR else 0o644
    else:
        raise RuntimeError(f"Unsupported archive entry: {info.name}")
    return info


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as source:
        while chunk := source.read(64 * 1024):
            digest.update(chunk)
    return digest.hexdigest()


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--package-dir", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    arguments = parser.parse_args()
    checksum = create_archive(arguments.package_dir, arguments.output)
    print(f"Built Zeta Code archive at {arguments.output.resolve()}")
    print(f"Built checksum at {checksum}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
