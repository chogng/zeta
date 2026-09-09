"""Sign one staged app package with its system release signer."""

from __future__ import annotations

import argparse
from pathlib import Path

from app_signing import record_verified_package, sign_package


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--package-dir", type=Path, required=True)
    parser.add_argument("--verify-only", action="store_true")
    args = parser.parse_args()
    record = (
        record_verified_package(args.package_dir)
        if args.verify_only
        else sign_package(args.package_dir)
    )
    print(f"Signed app package: {record['platform']} {record['signedSha256']}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
