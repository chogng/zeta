#!/usr/bin/env python3
"""Stable command-line entry point for the Zeta package builder."""

import sys
from pathlib import Path


REPOSITORY_ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPOSITORY_ROOT))

from zeta_package.cli import main


if __name__ == "__main__":
    raise SystemExit(main())
