"""Verify a signed app package before publication."""

from __future__ import annotations

import argparse
from pathlib import Path

from app_signing import host_platform, verify_package


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--package-dir", type=Path, required=True)
    parser.add_argument(
        "--platform", choices=["darwin", "linux", "windows"], default=host_platform()
    )
    args = parser.parse_args()
    record = verify_package(args.package_dir, args.platform)
    print(f"Verified app package: {record['platform']} {record['verifiedSha256']}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
