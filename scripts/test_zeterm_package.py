import json
import os
import tempfile
import unittest
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

from build_zeterm_package import build_package
from build_zeterm_package import remote_runtime_network_release
from remote_runtime_bundle import build_remote_runtime_bundle
from test_remote_runtime_bundle import create_package


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

    def test_stages_a_catalog_bound_remote_runtime_bundle(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            binary = root / "zeterm"
            binary.write_bytes(b"zeterm-test-binary")
            binary.chmod(0o755)
            bundle = build_remote_runtime_bundle(
                root / "remote-runtimes", [create_package(root / "runtime-package")]
            )
            binary.write_bytes(b"zeterm-test-binary:" + bundle.catalog_sha256.encode())
            output = root / "package"

            build_package(
                output,
                binary,
                "aarch64-apple-darwin",
                "release",
                bundle,
            )

            metadata = json.loads((output / "zeterm-package.json").read_text())
            binding = metadata["remoteRuntimeCatalog"]
            self.assertEqual(bundle.catalog_sha256, binding["sha256"])
            self.assertEqual("compiledIntoSignedBinary", binding["trustBinding"])
            self.assertTrue((output / binding["path"]).is_file())

    def test_rejects_a_remote_bundle_not_bound_into_the_binary(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            binary = root / "zeterm"
            binary.write_bytes(b"zeterm-test-binary")
            binary.chmod(0o755)
            bundle = build_remote_runtime_bundle(
                root / "remote-runtimes", [create_package(root / "runtime-package")]
            )

            with self.assertRaisesRegex(RuntimeError, "does not contain"):
                build_package(
                    root / "package",
                    binary,
                    "aarch64-apple-darwin",
                    "release",
                    bundle,
                )

    def test_stages_a_network_catalog_bound_into_the_binary(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            release = remote_runtime_network_release(
                "https://releases.example/zeta/catalog.json", "a" * 64
            )
            binary = root / "zeterm"
            binary.write_bytes(
                b"zeterm-test-binary:"
                + release.catalog_sha256.encode()
                + b":"
                + release.url.encode()
            )
            binary.chmod(0o755)
            output = root / "package"

            build_package(
                output,
                binary,
                "aarch64-apple-darwin",
                "release",
                remote_runtime_release=release,
            )

            metadata = json.loads((output / "zeterm-package.json").read_text())
            self.assertEqual(
                {
                    "url": release.url,
                    "sha256": release.catalog_sha256,
                    "trustBinding": "compiledIntoSignedBinary",
                },
                metadata["remoteRuntimeCatalog"],
            )
            self.assertFalse((output / "zeta-remote-runtimes").exists())

    def test_rejects_an_invalid_network_catalog_release(self) -> None:
        with self.assertRaisesRegex(RuntimeError, "credential-free HTTPS"):
            remote_runtime_network_release(
                "https://user@releases.example/zeta/catalog.json", "a" * 64
            )


if __name__ == "__main__":
    unittest.main()
