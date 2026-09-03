#!/usr/bin/env python3
"""Run the repository-owned Python test suites."""

from __future__ import annotations

import os
import subprocess
import sys
from pathlib import Path


REPOSITORY_ROOT = Path(__file__).resolve().parents[1]
TEST_ROOTS = (
    "scripts",
    "build/lib/zeta_build",
    "build/release",
)


def main() -> int:
    environment = os.environ.copy()
    python_paths = [
        str(REPOSITORY_ROOT),
        str(REPOSITORY_ROOT / "build" / "release"),
    ]
    if existing := environment.get("PYTHONPATH"):
        python_paths.append(existing)
    environment["PYTHONPATH"] = os.pathsep.join(python_paths)

    for test_root in TEST_ROOTS:
        result = subprocess.run(
            [
                sys.executable,
                "-B",
                "-m",
                "unittest",
                "discover",
                "-s",
                test_root,
                "-p",
                "test_*.py",
            ],
            cwd=REPOSITORY_ROOT,
            env=environment,
            check=False,
        )
        if result.returncode != 0:
            return result.returncode
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
