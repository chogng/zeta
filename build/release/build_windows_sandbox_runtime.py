#!/usr/bin/env python3
"""Build the machine-wide Zeta Windows sandbox runtime MSI."""

import argparse
import sys
from pathlib import Path


REPOSITORY_ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPOSITORY_ROOT))
sys.path.insert(0, str(REPOSITORY_ROOT / "build" / "release"))

from build.lib.zeta_build.targets import TARGETS
from windows_sandbox_runtime import build_runtime_msi
from windows_sandbox_runtime import prepare_runtime_msi

WINDOWS_TARGETS = {target: spec for target, spec in TARGETS.items() if spec.is_windows}


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--package-dir", type=Path, required=True)
    parser.add_argument("--output-dir", type=Path, required=True)
    parser.add_argument("--target", choices=sorted(WINDOWS_TARGETS), required=True)
    parser.add_argument("--wix", default="wix")
    args = parser.parse_args()
    plan = prepare_runtime_msi(
        args.package_dir,
        WINDOWS_TARGETS[args.target],
        args.output_dir,
        args.wix,
    )
    artifact = build_runtime_msi(plan)
    print(f"Built Zeta Windows sandbox runtime at {artifact}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
