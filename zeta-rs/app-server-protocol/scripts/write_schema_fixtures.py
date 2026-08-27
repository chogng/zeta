#!/usr/bin/env python3
"""Regenerate the checked-in App Server protocol fixtures."""

import subprocess
from pathlib import Path


CRATE_DIRECTORY = Path(__file__).resolve().parents[1]
REPOSITORY_DIRECTORY = CRATE_DIRECTORY.parents[1]


def generate(artifact: str, output_directory: Path) -> None:
    subprocess.run(
        [
            "cargo",
            "run",
            "--quiet",
            "--manifest-path",
            str(REPOSITORY_DIRECTORY / "Cargo.toml"),
            "-p",
            "zeta-app-server-protocol",
            "--bin",
            "generate_protocol",
            "--",
            artifact,
            "--out",
            str(output_directory),
        ],
        cwd=REPOSITORY_DIRECTORY,
        check=True,
    )


def main() -> None:
    generate("json", CRATE_DIRECTORY / "schema" / "json")
    generate("typescript", CRATE_DIRECTORY / "schema" / "typescript")


if __name__ == "__main__":
    main()
