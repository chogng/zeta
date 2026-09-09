#!/usr/bin/env python3
"""Submit one final macOS release container for notarization."""

from __future__ import annotations

import argparse
from pathlib import Path

from system_signing import notarize


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("artifact", type=Path)
    parser.add_argument("--staple", action="store_true")
    arguments = parser.parse_args()
    notarize(arguments.artifact, staple=arguments.staple)
    print(f"Notarized {arguments.artifact.resolve()}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
