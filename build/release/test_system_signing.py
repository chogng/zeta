from __future__ import annotations

import os
import sys
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parent))

from sign_zeta_package import sign_package
from system_signing import sha256
from system_signing import notarize, sign_and_verify


class SystemSigningTests(unittest.TestCase):
    def test_macos_signs_with_hardened_runtime_and_timestamp(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            artifact = Path(temporary) / "zeta"
            artifact.write_bytes(b"unsigned")
            commands = []

            def runner(command):
                commands.append(list(command))
                if command[1] == "--force":
                    artifact.write_bytes(b"signed")

            with patch.dict(
                os.environ,
                {"ZETA_MACOS_SIGNING_IDENTITY": "Developer ID Application: Zeta"},
                clear=False,
            ):
                result = sign_and_verify(artifact, "darwin", runner)

            self.assertNotEqual(result.unsigned_sha256, result.signed_sha256)
            self.assertEqual("--timestamp", commands[0][4])
            self.assertEqual(["--options", "runtime"], commands[0][5:7])
            self.assertEqual(["--verify", "--strict", "--verbose=2"], commands[1][1:4])

    def test_windows_signs_by_certificate_thumbprint_with_rfc3161_timestamp(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            artifact = Path(temporary) / "zeta.exe"
            artifact.write_bytes(b"unsigned")
            commands = []
            with patch.dict(
                os.environ,
                {"ZETA_WINDOWS_SIGNING_THUMBPRINT": "ABC123"},
                clear=False,
            ):
                sign_and_verify(
                    artifact, "windows", lambda command: commands.append(list(command))
                )

            self.assertEqual(
                [
                    "signtool",
                    "sign",
                    "/fd",
                    "SHA256",
                    "/sha1",
                    "ABC123",
                    "/tr",
                    "http://timestamp.digicert.com",
                    "/td",
                    "SHA256",
                    str(artifact.resolve()),
                ],
                commands[0],
            )
            self.assertEqual(["verify", "/pa", "/all", "/v"], commands[1][1:5])

    def test_macos_notarization_uses_a_keychain_profile_and_can_staple(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            artifact = Path(temporary) / "Zeta.pkg"
            artifact.write_bytes(b"package")
            commands = []
            with patch.dict(
                os.environ, {"ZETA_MACOS_NOTARY_PROFILE": "zeta-release"}, clear=False
            ):
                notarize(artifact, lambda command: commands.append(list(command)), staple=True)

            self.assertEqual("notarytool", commands[0][1])
            self.assertEqual("zeta-release", commands[0][-2])
            self.assertEqual("staple", commands[1][2])
            self.assertEqual("validate", commands[2][2])

    def test_managed_windows_signature_is_verified_before_package_hashes_refresh(
        self,
    ) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            package = Path(temporary).resolve()
            artifact = package / "bin/zeta.exe"
            artifact.parent.mkdir()
            artifact.write_bytes(b"signed-by-service")
            unsigned_digest = "1" * 64
            (package / "zeta-package.json").write_text(
                '{"target":"x86_64-pc-windows-msvc","files":{"bin/zeta.exe":"'
                + unsigned_digest
                + '"}}',
                encoding="utf-8",
            )
            commands = []
            with patch(
                "sign_zeta_package.system_signing_artifacts",
                return_value={"cli": artifact},
            ), patch("sign_zeta_package.record_system_signing") as record:
                sign_package(
                    package,
                    "x86_64-pc-windows-msvc",
                    lambda command: commands.append(list(command)),
                    verify_only=True,
                )

            self.assertEqual("verify", commands[0][1])
            self.assertEqual(
                {
                    "cli": {
                        "unsignedSha256": unsigned_digest,
                        "signedSha256": sha256(artifact),
                    }
                },
                record.call_args.args[2],
            )


if __name__ == "__main__":
    unittest.main()
