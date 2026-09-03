from __future__ import annotations

import json
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

import zeta


class ZetaLauncherTests(unittest.TestCase):
    def test_current_package_resolves_the_latest_numbered_manifest(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            development_root = Path(temporary)
            build = "a" * 64
            package_root = development_root / "packages" / "0.1.0" / build
            package_root.mkdir(parents=True)
            manifests = development_root / "manifests"
            manifests.mkdir()
            (manifests / "00000000000000000001.json").write_text(
                json.dumps({"formatVersion": 1, "sequence": 1, "directory": f"packages/0.1.0/{build}"}),
                encoding="utf-8",
            )

            with patch.object(zeta, "development_root", return_value=development_root):
                self.assertEqual(zeta.current_package(), package_root)

    def test_current_package_rejects_a_path_outside_the_package_store(
        self,
    ) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            development_root = Path(temporary)
            manifests = development_root / "manifests"
            manifests.mkdir()
            (manifests / "00000000000000000001.json").write_text(
                json.dumps({"formatVersion": 1, "sequence": 1, "directory": "../package"}),
                encoding="utf-8",
            )

            with patch.object(zeta, "development_root", return_value=development_root):
                with self.assertRaisesRegex(
                    RuntimeError, "invalid Zeta development package manifest"
                ):
                    zeta.current_package()


if __name__ == "__main__":
    unittest.main()
