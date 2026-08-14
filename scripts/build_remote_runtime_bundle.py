#!/usr/bin/env python3
"""Build deterministic Remote runtime archives and their host-authenticated catalog."""

from __future__ import annotations

import argparse
from pathlib import Path

from remote_runtime_bundle import build_remote_runtime_bundle


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--bundle-dir", type=Path, required=True)
    parser.add_argument("--package-dir", type=Path, action="append", required=True)
    args = parser.parse_args()
    bundle = build_remote_runtime_bundle(args.bundle_dir, args.package_dir)
    print(f"Built Remote runtime bundle at {bundle.root}")
    print(f"Catalog SHA-256: {bundle.catalog_sha256}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
