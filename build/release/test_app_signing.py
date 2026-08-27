import json
import os
import sys
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parent))

from build_app_package import build_package
from build_app_package import remote_runtime_network_release
from remote_runtime_bundle import build_remote_runtime_bundle
from test_remote_runtime_bundle import create_package
from app_signing import sign_package, verify_package


class AppSigningTests(unittest.TestCase):
    def test_linux_sign_and_verify_update_release_state(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            binary = root / "app"
            binary.write_bytes(b"app-signing-test")
            binary.chmod(0o755)
            package = root / "package"
            build_package(package, binary, "x86_64-unknown-linux-gnu", "release")
            commands = []

            def fake_runner(command):
                commands.append(list(command))
                if command[0] == "cosign" and command[1] == "sign-blob":
                    output = Path(command[command.index("--output-signature") + 1])
                    output.write_bytes(b"detached-signature")

            with patch.dict(os.environ, {"APP_COSIGN_IDENTITY": "test-key"}, clear=False):
                signed = sign_package(package, "linux", fake_runner)
                verified = verify_package(package, "linux", fake_runner)

            self.assertEqual("signed", signed["status"])
            self.assertEqual("verified", verified["status"])
            self.assertEqual(2, len(commands))
            self.assertTrue((package / "app-signature.sig").is_file())
            metadata = json.loads((package / "app-package.json").read_text())
            record = json.loads((package / "app-signature.json").read_text())
            self.assertEqual("verified", metadata["signing"]["status"])
            self.assertEqual(metadata["binary"]["sha256"], record["verifiedSha256"])

    def test_sign_rejects_a_tampered_staged_binary(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            binary = root / "app"
            binary.write_bytes(b"app-signing-test")
            binary.chmod(0o755)
            package = root / "package"
            build_package(package, binary, "x86_64-unknown-linux-gnu", "release")
            (package / "bin" / "app").write_bytes(b"tampered")

            with patch.dict(os.environ, {"APP_COSIGN_IDENTITY": "test-key"}, clear=False):
                with self.assertRaisesRegex(RuntimeError, "digest"):
                    sign_package(package, "linux", lambda _: None)

    def test_signature_record_binds_the_compiled_remote_runtime_catalog(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            binary = root / "app"
            binary.write_bytes(b"app-signing-test")
            binary.chmod(0o755)
            bundle = build_remote_runtime_bundle(
                root / "remote-runtimes", [create_package(root / "runtime-package")]
            )
            binary.write_bytes(b"app-signing-test:" + bundle.catalog_sha256.encode())
            package = root / "package"
            build_package(
                package,
                binary,
                "x86_64-unknown-linux-gnu",
                "release",
                bundle,
            )

            def fake_runner(command):
                if command[0] == "cosign" and command[1] == "sign-blob":
                    Path(command[command.index("--output-signature") + 1]).write_bytes(
                        b"detached-signature"
                    )

            with patch.dict(os.environ, {"APP_COSIGN_IDENTITY": "test-key"}, clear=False):
                signed = sign_package(package, "linux", fake_runner)
                verified = verify_package(package, "linux", fake_runner)

            self.assertEqual(bundle.catalog_sha256, signed["remoteRuntimeCatalogSha256"])
            self.assertEqual(bundle.catalog_sha256, verified["remoteRuntimeCatalogSha256"])

    def test_signature_record_binds_a_network_remote_runtime_catalog(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            release = remote_runtime_network_release(
                "https://releases.example/zeta/catalog.json", "b" * 64
            )
            binary = root / "app"
            binary.write_bytes(
                b"app-signing-test:"
                + release.catalog_sha256.encode()
                + b":"
                + release.url.encode()
            )
            binary.chmod(0o755)
            package = root / "package"
            build_package(
                package,
                binary,
                "x86_64-unknown-linux-gnu",
                "release",
                remote_runtime_release=release,
            )

            def fake_runner(command):
                if command[0] == "cosign" and command[1] == "sign-blob":
                    Path(command[command.index("--output-signature") + 1]).write_bytes(
                        b"detached-signature"
                    )

            with patch.dict(os.environ, {"APP_COSIGN_IDENTITY": "test-key"}, clear=False):
                signed = sign_package(package, "linux", fake_runner)
                verified = verify_package(package, "linux", fake_runner)

            self.assertEqual(release.catalog_sha256, signed["remoteRuntimeCatalogSha256"])
            self.assertEqual(release.catalog_sha256, verified["remoteRuntimeCatalogSha256"])


if __name__ == "__main__":
    unittest.main()
