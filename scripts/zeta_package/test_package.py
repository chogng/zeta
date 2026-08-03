import hashlib
import io
import json
import os
import stat
import tarfile
import tempfile
import unittest
import zipfile
from pathlib import Path

from zeta_package.bubblewrap import load_source_lock, resolve_bubblewrap
from zeta_package.layout import build_package_directory, copy_builtin_skills
from zeta_package.ripgrep import load_lock, resolve_ripgrep
from zeta_package.targets import TARGETS
from zeta_package.version import read_workspace_version
from zeta_package.windows_helpers import resolve_windows_sandbox_helpers


REPOSITORY_ROOT = Path(__file__).resolve().parents[2]
PRODUCTION_LOCK = REPOSITORY_ROOT / "third_party" / "ripgrep" / "runtime-lock.json"
PRODUCTION_BUBBLEWRAP_LOCK = (
    REPOSITORY_ROOT / "third_party" / "bubblewrap" / "runtime-lock.json"
)


class PackageTests(unittest.TestCase):
    def test_production_lock_covers_every_package_target(self) -> None:
        lock = load_lock(PRODUCTION_LOCK)

        self.assertEqual(set(TARGETS), set(lock["packageTargets"]))
        for artifact_key in lock["packageTargets"].values():
            artifact = lock["artifacts"][artifact_key]
            self.assertEqual(64, len(artifact["sha256"]))
            self.assertIn(artifact["format"], ("tar.gz", "zip"))

        bubblewrap_lock = load_source_lock(PRODUCTION_BUBBLEWRAP_LOCK)
        self.assertEqual("0.11.2", bubblewrap_lock.version)
        self.assertEqual(64, len(bubblewrap_lock.archive_sha256))

    def test_local_overrides_build_canonical_package(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            zeta_binary = executable_file(root / "zeta-source", b"zeta")
            rg_binary = executable_file(root / "rg-source", b"ripgrep")
            output = root / "package"
            spec = TARGETS["aarch64-apple-darwin"]
            ripgrep = resolve_ripgrep(
                spec,
                PRODUCTION_LOCK,
                root / "cache",
                explicit_binary=rg_binary,
            )

            build_package_directory(
                output,
                REPOSITORY_ROOT,
                read_workspace_version(REPOSITORY_ROOT / "Cargo.toml"),
                spec,
                zeta_binary,
                ripgrep,
            )

            self.assertEqual(b"zeta", (output / "bin" / "zeta").read_bytes())
            self.assertEqual(b"ripgrep", (output / "zeta-path" / "rg").read_bytes())
            self.assertTrue(os.access(str(output / "bin" / "zeta"), os.X_OK))
            self.assertTrue(os.access(str(output / "zeta-path" / "rg"), os.X_OK))
            self.assertTrue(
                (
                    output
                    / "zeta-resources"
                    / "licenses"
                    / "ripgrep"
                    / "LICENSE-MIT"
                ).is_file()
            )
            self.assertEqual(
                (
                    REPOSITORY_ROOT
                    / "zeta-rs"
                    / "skills"
                    / "assets"
                    / "skill-creator"
                    / "SKILL.md"
                ).read_text(encoding="utf-8"),
                (
                    output
                    / "zeta-resources"
                    / "skills"
                    / "skill-creator"
                    / "SKILL.md"
                ).read_text(encoding="utf-8"),
            )
            metadata = json.loads(
                (output / "zeta-package.json").read_text(encoding="utf-8")
            )
            self.assertEqual("aarch64-apple-darwin", metadata["target"])
            self.assertEqual("local-override", metadata["components"]["ripgrep"]["source"])
            self.assertEqual(
                hashlib.sha256(b"ripgrep").hexdigest(),
                metadata["components"]["ripgrep"]["binarySha256"],
            )

            with self.assertRaisesRegex(RuntimeError, "Refusing to replace"):
                build_package_directory(
                    output,
                    REPOSITORY_ROOT,
                    "0.1.0",
                    spec,
                    zeta_binary,
                    ripgrep,
                )

    def test_linux_package_contains_built_sandbox_resource_contract(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            source_archive = root / "bubblewrap-test.tar.xz"
            write_bubblewrap_source_archive(source_archive)
            bubblewrap_lock = write_bubblewrap_lock(root, source_archive)
            zeta_binary = executable_file(root / "zeta-source", b"zeta")
            rg_binary = executable_file(root / "rg-source", b"ripgrep")
            bwrap_binary = executable_file(root / "bwrap-source", b"bubblewrap")
            spec = TARGETS["x86_64-unknown-linux-musl"]
            ripgrep = resolve_ripgrep(
                spec,
                PRODUCTION_LOCK,
                root / "rg-cache",
                explicit_binary=rg_binary,
            )
            bubblewrap = resolve_bubblewrap(
                REPOSITORY_ROOT,
                spec,
                bubblewrap_lock,
                root / "bubblewrap-cache",
                explicit_binary=bwrap_binary,
                cargo="cargo",
                cargo_profile="release",
            )
            self.assertIsNotNone(bubblewrap)
            output = root / "package"

            build_package_directory(
                output,
                REPOSITORY_ROOT,
                "0.1.0",
                spec,
                zeta_binary,
                ripgrep,
                bubblewrap,
            )

            self.assertEqual(
                b"bubblewrap",
                (output / "zeta-resources" / "bwrap").read_bytes(),
            )
            self.assertTrue(
                (
                    output
                    / "zeta-resources"
                    / "licenses"
                    / "bubblewrap"
                    / "COPYING"
                ).is_file()
            )
            metadata = json.loads(
                (output / "zeta-package.json").read_text(encoding="utf-8")
            )
            self.assertEqual(
                "0.11.2-test",
                metadata["components"]["bubblewrap"]["version"],
            )
            self.assertEqual(
                "local-override",
                metadata["components"]["bubblewrap"]["source"],
            )

    def test_windows_package_contains_both_sandbox_helpers(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            zeta_binary = root / "zeta.exe"
            zeta_binary.write_bytes(b"zeta")
            rg_binary = root / "rg.exe"
            rg_binary.write_bytes(b"ripgrep")
            command_runner = root / "zeta-command-runner.exe"
            command_runner.write_bytes(b"runner")
            sandbox_setup = root / "zeta-windows-sandbox-setup.exe"
            sandbox_setup.write_bytes(b"setup")
            spec = TARGETS["x86_64-pc-windows-msvc"]
            ripgrep = resolve_ripgrep(
                spec,
                PRODUCTION_LOCK,
                root / "rg-cache",
                explicit_binary=rg_binary,
            )
            helpers = resolve_windows_sandbox_helpers(
                REPOSITORY_ROOT,
                spec,
                command_runner,
                sandbox_setup,
                cargo="cargo",
                cargo_profile="release",
            )
            self.assertIsNotNone(helpers)
            output = root / "package"

            build_package_directory(
                output,
                REPOSITORY_ROOT,
                "0.1.0",
                spec,
                zeta_binary,
                ripgrep,
                windows_helpers=helpers,
            )

            resources = output / "zeta-resources"
            self.assertEqual(
                b"runner",
                (resources / "zeta-command-runner.exe").read_bytes(),
            )
            self.assertEqual(
                b"setup",
                (resources / "zeta-windows-sandbox-setup.exe").read_bytes(),
            )
            metadata = json.loads(
                (output / "zeta-package.json").read_text(encoding="utf-8")
            )
            component = metadata["components"]["windowsSandbox"]
            self.assertEqual("local-override", component["source"])
            self.assertEqual(
                hashlib.sha256(b"runner").hexdigest(),
                component["commandRunnerSha256"],
            )
            self.assertEqual(
                hashlib.sha256(b"setup").hexdigest(),
                component["sandboxSetupSha256"],
            )

    def test_windows_helper_overrides_are_rejected_for_other_targets(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            helper = executable_file(root / "helper", b"helper")

            with self.assertRaisesRegex(RuntimeError, "only supported for Windows"):
                resolve_windows_sandbox_helpers(
                    REPOSITORY_ROOT,
                    TARGETS["aarch64-apple-darwin"],
                    helper,
                    helper,
                    cargo="cargo",
                    cargo_profile="release",
                )

    @unittest.skipIf(os.name == "nt", "creating symbolic links may require Windows privilege")
    def test_builtin_skill_copy_rejects_symbolic_links(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            source = root / "source"
            skill = source / "review"
            skill.mkdir(parents=True)
            (skill / "SKILL.md").write_text(
                "---\nname: review\ndescription: Reviews code when requested.\n---\n",
                encoding="utf-8",
            )
            (skill / "linked.md").symlink_to(skill / "SKILL.md")

            with self.assertRaisesRegex(RuntimeError, "symbolic link"):
                copy_builtin_skills(source, root / "destination")

    def test_bubblewrap_source_digest_mismatch_aborts(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            source_archive = root / "bubblewrap-test.tar.xz"
            write_bubblewrap_source_archive(source_archive)
            lock_path = write_bubblewrap_lock(root, source_archive)
            lock = json.loads(lock_path.read_text(encoding="utf-8"))
            lock["archive"]["sha256"] = "0" * 64
            lock_path.write_text(json.dumps(lock), encoding="utf-8")
            bwrap_binary = executable_file(root / "bwrap-source", b"bubblewrap")

            with self.assertRaisesRegex(RuntimeError, "SHA-256"):
                resolve_bubblewrap(
                    REPOSITORY_ROOT,
                    TARGETS["x86_64-unknown-linux-musl"],
                    lock_path,
                    root / "cache",
                    explicit_binary=bwrap_binary,
                    cargo="cargo",
                    cargo_profile="release",
                )

            self.assertFalse(
                (
                    root
                    / "cache"
                    / "0.11.2-test"
                    / source_archive.name
                ).exists()
            )

    def test_bubblewrap_lock_rejects_source_path_escape(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            source_archive = root / "bubblewrap-test.tar.xz"
            write_bubblewrap_source_archive(source_archive)
            lock_path = write_bubblewrap_lock(root, source_archive)
            lock = json.loads(lock_path.read_text(encoding="utf-8"))
            lock["archive"]["members"].append("../escape")
            lock_path.write_text(json.dumps(lock), encoding="utf-8")

            with self.assertRaisesRegex(RuntimeError, "safe relative path"):
                load_source_lock(lock_path)

    def test_fetches_and_extracts_exact_tar_member(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            archive = root / "ripgrep-test.tar.gz"
            write_tar_archive(archive, "bundle/rg", b"tar-rg")
            lock_path = write_test_lock(
                root, archive, "tar.gz", "bundle/rg", "x86_64-unknown-linux-musl"
            )

            resolution = resolve_ripgrep(
                TARGETS["x86_64-unknown-linux-gnu"],
                lock_path,
                root / "cache",
            )

            self.assertEqual(b"tar-rg", resolution.executable.read_bytes())
            self.assertTrue(os.access(str(resolution.executable), os.X_OK))
            self.assertEqual("upstream-release", resolution.source)

    def test_fetches_and_extracts_exact_zip_member(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            archive = root / "ripgrep-test.zip"
            with zipfile.ZipFile(str(archive), "w") as output:
                output.writestr("bundle/rg.exe", b"zip-rg")
            lock_path = write_test_lock(
                root, archive, "zip", "bundle/rg.exe", "x86_64-pc-windows-msvc"
            )

            resolution = resolve_ripgrep(
                TARGETS["x86_64-pc-windows-msvc"],
                lock_path,
                root / "cache",
            )

            self.assertEqual(b"zip-rg", resolution.executable.read_bytes())
            self.assertEqual("rg.exe", resolution.executable.name)

    def test_digest_mismatch_aborts_and_removes_download(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            archive = root / "ripgrep-test.tar.gz"
            write_tar_archive(archive, "bundle/rg", b"tampered")
            lock_path = write_test_lock(
                root,
                archive,
                "tar.gz",
                "bundle/rg",
                "x86_64-unknown-linux-musl",
                digest="0" * 64,
            )
            cache = root / "cache"

            with self.assertRaisesRegex(RuntimeError, "SHA-256"):
                resolve_ripgrep(
                    TARGETS["x86_64-unknown-linux-musl"],
                    lock_path,
                    cache,
                )

            self.assertFalse(
                (
                    cache
                    / "test"
                    / "x86_64-unknown-linux-musl"
                    / archive.name
                ).exists()
            )


def executable_file(path: Path, contents: bytes) -> Path:
    path.write_bytes(contents)
    path.chmod(path.stat().st_mode | stat.S_IXUSR)
    return path


def write_tar_archive(path: Path, member_name: str, contents: bytes) -> None:
    with tarfile.open(str(path), "w:gz") as archive:
        member = tarfile.TarInfo(member_name)
        member.size = len(contents)
        member.mode = 0o755
        archive.addfile(member, io.BytesIO(contents))


def write_bubblewrap_source_archive(path: Path) -> None:
    members = {
        "COPYING": b"copying",
        "bind-mount.c": b"source",
        "bind-mount.h": b"header",
        "bubblewrap.c": b"source",
        "network.c": b"source",
        "network.h": b"header",
        "utils.c": b"source",
        "utils.h": b"header",
    }
    with tarfile.open(str(path), "w:xz") as archive:
        for name, contents in members.items():
            member = tarfile.TarInfo("bubblewrap-test/" + name)
            member.size = len(contents)
            archive.addfile(member, io.BytesIO(contents))


def write_bubblewrap_lock(root: Path, archive: Path) -> Path:
    lock = {
        "schemaVersion": 1,
        "runtime": "bubblewrap-source",
        "version": "0.11.2-test",
        "source": {
            "repository": "https://example.invalid/bubblewrap",
            "release": "test",
        },
        "archive": {
            "name": archive.name,
            "size": archive.stat().st_size,
            "sha256": hashlib.sha256(archive.read_bytes()).hexdigest(),
            "format": "tar.xz",
            "root": "bubblewrap-test",
            "members": [
                "COPYING",
                "bind-mount.c",
                "bind-mount.h",
                "bubblewrap.c",
                "network.c",
                "network.h",
                "utils.c",
                "utils.h",
            ],
            "url": archive.as_uri(),
        },
    }
    lock_path = root / "bubblewrap-lock.json"
    lock_path.write_text(json.dumps(lock), encoding="utf-8")
    return lock_path


def write_test_lock(
    root: Path,
    archive: Path,
    archive_format: str,
    executable_member: str,
    artifact_target: str,
    digest: str = "",
) -> Path:
    actual_digest = hashlib.sha256(archive.read_bytes()).hexdigest()
    lock = {
        "schemaVersion": 1,
        "runtime": "ripgrep",
        "version": "test",
        "source": {
            "repository": "https://example.invalid/ripgrep",
            "release": "test",
        },
        "packageTargets": {
            "x86_64-unknown-linux-gnu": artifact_target,
            "x86_64-unknown-linux-musl": artifact_target,
            "x86_64-pc-windows-msvc": artifact_target,
        },
        "artifacts": {
            artifact_target: {
                "archive": archive.name,
                "size": archive.stat().st_size,
                "sha256": digest or actual_digest,
                "format": archive_format,
                "executable": executable_member,
                "url": archive.as_uri(),
            }
        },
    }
    lock_path = root / "runtime-lock.json"
    lock_path.write_text(json.dumps(lock), encoding="utf-8")
    return lock_path


if __name__ == "__main__":
    unittest.main()
