"""Read the Zeta workspace package version without third-party TOML dependencies."""

import re
from pathlib import Path


WORKSPACE_PACKAGE = re.compile(
    r"^\[workspace\.package\]\s*$.*?^version\s*=\s*\"([^\"]+)\"\s*$",
    re.MULTILINE | re.DOTALL,
)


def read_workspace_version(manifest_path: Path) -> str:
    match = WORKSPACE_PACKAGE.search(manifest_path.read_text(encoding="utf-8"))
    if match is None:
        raise RuntimeError(
            "Could not find [workspace.package].version in {}".format(manifest_path)
        )
    return match.group(1)
