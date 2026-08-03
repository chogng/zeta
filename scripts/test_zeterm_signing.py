import json
import os
import sys
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parent))

from build_zeterm_package import build_package
from zeterm_signing import sign_package, verify_package


class ZetermSigningTests(unittest.TestCase):
    def test_linux_sign_and_verify_update_release_state(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            binary = root / "zeterm"
            binary.write_bytes(b"zeterm-signing-test")
            binary.chmod(0o755)
            package = root / "package"
            build_package(package, binary, "x86_64-unknown-linux-gnu", "release")
            commands = []

            def fake_runner(command):
                commands.append(list(command))
                if command[0] == "cosign" and command[1] == "sign-blob":
                    output = Path(command[command.index("--output-signature") + 1])
                    output.write_bytes(b"detached-signature")

            with patch.dict(os.environ, {"ZETERM_COSIGN_IDENTITY": "test-key"}, clear=False):
                signed = sign_package(package, "linux", fake_runner)
                verified = verify_package(package, "linux", fake_runner)

            self.assertEqual("signed", signed["status"])
            self.assertEqual("verified", verified["status"])
            self.assertEqual(2, len(commands))
            self.assertTrue((package / "zeterm-signature.sig").is_file())
            metadata = json.loads((package / "zeterm-package.json").read_text())
            record = json.loads((package / "zeterm-signature.json").read_text())
            self.assertEqual("verified", metadata["signing"]["status"])
            self.assertEqual(metadata["binary"]["sha256"], record["verifiedSha256"])

    def test_sign_rejects_a_tampered_staged_binary(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            binary = root / "zeterm"
            binary.write_bytes(b"zeterm-signing-test")
            binary.chmod(0o755)
            package = root / "package"
            build_package(package, binary, "x86_64-unknown-linux-gnu", "release")
            (package / "bin" / "zeterm").write_bytes(b"tampered")

            with patch.dict(os.environ, {"ZETERM_COSIGN_IDENTITY": "test-key"}, clear=False):
                with self.assertRaisesRegex(RuntimeError, "digest"):
                    sign_package(package, "linux", lambda _: None)


if __name__ == "__main__":
    unittest.main()
