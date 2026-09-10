"""Verify the review patch against an exact, clean Microsoft MXC checkout."""

from __future__ import annotations

import argparse
import difflib
import json
from pathlib import Path
import subprocess


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--upstream", type=Path, required=True)
    parser.add_argument("--write-patch", action="store_true")
    args = parser.parse_args()
    root = Path(__file__).resolve().parent
    origin = args.upstream.resolve()
    metadata = json.loads((root / "upstream.json").read_text())
    revision = subprocess.check_output(
        ["git", "-C", str(origin), "rev-parse", "HEAD"], text=True
    ).strip()
    if revision != metadata["revision"]:
        raise SystemExit("MXC checkout does not match the pinned revision")
    packages = list(metadata["packages"].values())
    for package in packages:
        path = Path(package)
        if path.is_absolute() or ".." in path.parts:
            raise SystemExit("invalid package path in upstream metadata")
    subprocess.run(
        [
            "git",
            "-C",
            str(origin),
            "diff",
            "--exit-code",
            "HEAD",
            "--",
            *["src/" + package for package in packages],
        ],
        check=True,
        stdout=subprocess.DEVNULL,
    )
    chunks: list[str] = []
    changed: list[str] = []
    for package in packages:
        before = origin / "src" / package
        after = root / package
        paths = sorted(
            {p.relative_to(before) for p in before.rglob("*") if p.is_file()}
            | {p.relative_to(after) for p in after.rglob("*") if p.is_file()}
        )
        renames = {}
        for old in before.rglob("mod.rs"):
            source = old.relative_to(before)
            target = source.parent.with_suffix(".rs")
            if not (after / source).exists() and (after / target).is_file():
                renames[source] = target
        for path in paths:
            if path in renames.values():
                continue
            target = renames.get(path, path)
            old, new = before / path, after / target
            if old.is_symlink() or new.is_symlink():
                raise SystemExit(
                    "linked files are not permitted in the SDK source snapshot"
                )
            a = old.read_text() if old.exists() else ""
            b = new.read_text() if new.exists() else ""
            name = (Path("src") / package / path).as_posix()
            destination_name = (Path("src") / package / target).as_posix()
            if path in renames:
                chunks.append(
                    f"diff --git a/{name} b/{destination_name}\nrename from {name}\nrename to {destination_name}\n"
                )
            if a == b and path not in renames:
                continue
            changed.append(destination_name)
            chunks.extend(
                difflib.unified_diff(
                    a.splitlines(True),
                    b.splitlines(True),
                    fromfile="a/" + name if old.exists() else "/dev/null",
                    tofile="b/" + destination_name if new.exists() else "/dev/null",
                )
            )
    patch = "".join(chunks)
    destination = root / "changes.patch"
    if args.write_patch:
        destination.write_text(patch)
    elif not destination.exists() or destination.read_text() != patch:
        raise SystemExit("MXC sources differ from the reviewed changes.patch")
    print(
        f"Verified MXC {revision}: {len(changed)} changed files across {len(packages)} existing SDK packages"
    )


if __name__ == "__main__":
    main()
