"""Sign one staged app package with the platform-native release signer."""

from __future__ import annotations

import argparse
from pathlib import Path

from app_signing import host_platform, sign_package


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--package-dir", type=Path, required=True)
    parser.add_argument("--platform", choices=["darwin", "linux", "windows"], default=host_platform())
    args = parser.parse_args()
    record = sign_package(args.package_dir, args.platform)
    print(f"Signed app package: {record['platform']} {record['signedSha256']}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
