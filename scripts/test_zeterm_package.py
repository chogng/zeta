import json
import os
import tempfile
import unittest
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

from build_zeterm_package import build_package


class ZetermPackageTests(unittest.TestCase):
    def test_stages_binary_digest_and_unsigned_state(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            binary = root / "zeterm"
            binary.write_bytes(b"zeterm-test-binary")
            binary.chmod(0o755)
            output = root / "package"

            build_package(output, binary, "aarch64-apple-darwin", "release")

            staged = output / "bin" / "zeterm"
            metadata = json.loads((output / "zeterm-package.json").read_text())
            self.assertEqual(binary.read_bytes(), staged.read_bytes())
            self.assertTrue((output / "zeterm-signing-policy.json").is_file())
            self.assertTrue(os.access(staged, os.X_OK))
            self.assertEqual("unsigned", metadata["signing"]["status"])
            self.assertTrue(metadata["signing"]["requiredForRelease"])
            self.assertEqual(64, len(metadata["binary"]["sha256"]))

    def test_refuses_to_replace_existing_output(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            binary = root / "zeterm"
            binary.write_bytes(b"zeterm-test-binary")
            output = root / "package"
            output.mkdir()

            with self.assertRaisesRegex(RuntimeError, "refusing to replace"):
                build_package(output, binary, "aarch64-apple-darwin", "release")


if __name__ == "__main__":
    unittest.main()
