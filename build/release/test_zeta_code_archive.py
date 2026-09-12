from __future__ import annotations

import importlib.util
import hashlib
import json
import os
import platform
import subprocess
import tarfile
import tempfile
import unittest
import zipfile
from pathlib import Path
from unittest.mock import patch


MODULE_PATH = Path(__file__).with_name("build_zeta_code_archive.py")
SPEC = importlib.util.spec_from_file_location("build_zeta_code_archive", MODULE_PATH)
assert SPEC is not None and SPEC.loader is not None
archive_builder = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(archive_builder)


class ZetaCodeArchiveTests(unittest.TestCase):
    def test_archive_is_rootless_deterministic_and_has_a_named_checksum(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            package = root / "package"
            executable = package / "bin/zeta"
            executable.parent.mkdir(parents=True)
            executable.write_bytes(b"zeta")
            executable.chmod(0o755)
            (package / "zeta-package.json").write_text(
                json.dumps(
                    {
                        "layoutVersion": 2,
                        "target": "aarch64-unknown-linux-gnu",
                        "components": {"cli": {}},
                    }
                ),
                encoding="utf-8",
            )
            first = root / "first/zeta-code-aarch64-unknown-linux-gnu.tar.gz"
            second = root / "second/zeta-code-aarch64-unknown-linux-gnu.tar.gz"

            with patch.object(archive_builder, "validate_package_directory"):
                checksum = archive_builder.create_archive(package, first)
                for path in package.rglob("*"):
                    os.utime(path, (86400, 86400))
                archive_builder.create_archive(package, second)

            self.assertEqual(first.read_bytes(), second.read_bytes())
            self.assertEqual(
                checksum.read_text(encoding="ascii"),
                f"{archive_builder.sha256(first)}  {first.name}\n",
            )
            with tarfile.open(first, "r:gz") as archive:
                self.assertEqual(
                    archive.getnames(),
                    ["bin", "bin/zeta", "zeta-package.json"],
                )
                self.assertEqual(archive.extractfile("bin/zeta").read(), b"zeta")
                self.assertEqual(archive.getmember("bin/zeta").mode, 0o755)
                self.assertEqual(archive.getmember("zeta-package.json").mode, 0o644)
                for member in archive.getmembers():
                    self.assertEqual((member.uid, member.gid, member.mtime), (0, 0, 0))
                    self.assertEqual((member.uname, member.gname), ("", ""))

    def test_macos_archive_is_a_deterministic_rootless_zip(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            package = root / "package"
            executable = package / "bin/zeta"
            executable.parent.mkdir(parents=True)
            executable.write_bytes(b"zeta")
            executable.chmod(0o755)
            (package / "zeta-package.json").write_text(
                json.dumps(
                    {
                        "layoutVersion": 2,
                        "target": "aarch64-apple-darwin",
                        "components": {"cli": {}},
                    }
                ),
                encoding="utf-8",
            )
            first = root / "first/zeta-code-aarch64-apple-darwin.zip"
            second = root / "second/zeta-code-aarch64-apple-darwin.zip"

            with (
                patch.object(archive_builder, "validate_package_directory"),
                patch.object(archive_builder, "require_verified_system_signing"),
            ):
                archive_builder.create_archive(package, first)
                archive_builder.create_archive(package, second)

            self.assertEqual(first.read_bytes(), second.read_bytes())
            with zipfile.ZipFile(first) as archive:
                self.assertEqual(
                    archive.namelist(), ["bin/", "bin/zeta", "zeta-package.json"]
                )

    def test_archive_requires_cli_package_identity_and_new_output(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            package = root / "package"
            package.mkdir()
            (package / "zeta-package.json").write_text(
                json.dumps(
                    {
                        "layoutVersion": 2,
                        "target": "aarch64-apple-darwin",
                        "components": {},
                    }
                ),
                encoding="utf-8",
            )
            output = root / "zeta-code-aarch64-apple-darwin.zip"
            with self.assertRaisesRegex(RuntimeError, "identity is invalid"):
                archive_builder.create_archive(package, output)

    def test_macos_archive_refuses_an_unsigned_package(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            package = root / "package"
            package.mkdir()
            (package / "zeta-package.json").write_text(
                json.dumps(
                    {
                        "layoutVersion": 2,
                        "target": "aarch64-apple-darwin",
                        "components": {"cli": {}},
                    }
                ),
                encoding="utf-8",
            )
            output = root / "zeta-code-aarch64-apple-darwin.zip"
            with patch.object(archive_builder, "validate_package_directory"):
                with self.assertRaisesRegex(RuntimeError, "system signing"):
                    archive_builder.create_archive(package, output)

    @unittest.skipIf(os.name == "nt", "the POSIX installer is tested on POSIX hosts")
    def test_posix_installer_publishes_a_version_and_stable_launcher(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            fixture = root / "fixture"
            executable = fixture / "bin/zeta"
            executable.parent.mkdir(parents=True)
            executable.write_text(
                "#!/bin/sh\nprintf 'zeta 1.2.3\\n'\n", encoding="utf-8"
            )
            executable.chmod(0o755)
            target = {
                ("Darwin", "arm64"): "aarch64-apple-darwin",
                ("Darwin", "x86_64"): "x86_64-apple-darwin",
                ("Linux", "aarch64"): "aarch64-unknown-linux-gnu",
                ("Linux", "x86_64"): "x86_64-unknown-linux-gnu",
            }[(platform.system(), platform.machine())]
            if platform.system() == "Darwin":
                archive = root / "package.zip"
                with zipfile.ZipFile(archive, "w") as output:
                    output.write(fixture / "bin", arcname="bin/")
                    output.write(executable, arcname="bin/zeta")
                release_name = f"zeta-code-{target}.zip"
            else:
                archive = root / "package.tar.gz"
                with tarfile.open(archive, "w:gz") as output:
                    output.add(fixture / "bin", arcname="bin")
                release_name = f"zeta-code-{target}.tar.gz"
            digest = hashlib.sha256(archive.read_bytes()).hexdigest()
            checksum = root / "package.sha256"
            checksum.write_text(f"{digest}  {release_name}\n", encoding="ascii")
            tools = root / "tools"
            tools.mkdir()
            curl = tools / "curl"
            curl.write_text(
                "#!/bin/sh\n"
                'while [ "$#" -gt 0 ]; do\n'
                '  if [ "$1" = -o ]; then shift; output=$1; fi\n'
                "  shift\n"
                "done\n"
                'case "$output" in\n'
                '  *.sha256) cp "$ZETA_TEST_CHECKSUM" "$output" ;;\n'
                '  *) cp "$ZETA_TEST_ARCHIVE" "$output" ;;\n'
                "esac\n",
                encoding="utf-8",
            )
            curl.chmod(0o755)
            install_root = root / "install"
            launcher_root = root / "launchers"
            environment = {
                **os.environ,
                "PATH": f"{tools}:/usr/bin:/bin",
                "ZETA_INSTALL_ROOT": str(install_root),
                "ZETA_BIN_DIR": str(launcher_root),
                "ZETA_TEST_ARCHIVE": str(archive),
                "ZETA_TEST_CHECKSUM": str(checksum),
            }

            result = subprocess.run(
                ["sh", "scripts/zeta-code/install.sh"],
                cwd=archive_builder.REPOSITORY_ROOT,
                env=environment,
                check=True,
                capture_output=True,
                text=True,
            )

            self.assertIn("Installed Zeta Code 1.2.3", result.stdout)
            selected = (install_root / "current").readlink()
            self.assertEqual(selected.parts[0], "versions")
            self.assertTrue((install_root / selected / "bin/zeta").is_file())
            self.assertEqual(
                (launcher_root / "zeta").readlink(),
                install_root / "current/bin/zeta",
            )


if __name__ == "__main__":
    unittest.main()
