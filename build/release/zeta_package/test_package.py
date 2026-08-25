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
from zeta_package.layout import (
    build_package_directory,
    copy_builtin_extensions,
    copy_builtin_skills,
)
from zeta_package.node import NodeResolution, artifact_for_target, load_node_lock, resolve_node
from zeta_package.ripgrep import load_lock, resolve_ripgrep
from zeta_package.targets import TARGETS
from zeta_package.version import read_workspace_version
from zeta_package.windows_helpers import resolve_windows_sandbox_helpers


REPOSITORY_ROOT = Path(__file__).resolve().parents[3]
PRODUCTION_LOCK = REPOSITORY_ROOT / "third_party" / "ripgrep" / "runtime-lock.json"
PRODUCTION_NODE_LOCK = REPOSITORY_ROOT / "third_party" / "node" / "runtime-lock.json"
PRODUCTION_BUBBLEWRAP_LOCK = (
    REPOSITORY_ROOT / "third_party" / "bubblewrap" / "runtime-lock.json"
)


class PackageTests(unittest.TestCase):
    BUILT_IN_EXTENSIONS = [
        "css",
        "html",
        "javascript",
        "json",
        "markdown-basics",
        "python",
        "rust",
        "shellscript",
        "sql",
        "theme-defaults",
        "typescript-basics",
        "xml",
        "yaml",
    ]

    def test_production_lock_covers_every_package_target(self) -> None:
        lock = load_lock(PRODUCTION_LOCK)

        self.assertEqual(set(TARGETS), set(lock["packageTargets"]))
        for artifact_key in lock["packageTargets"].values():
            artifact = lock["artifacts"][artifact_key]
            self.assertEqual(64, len(artifact["sha256"]))
            self.assertIn(artifact["format"], ("tar.gz", "zip"))

        node_lock = load_node_lock(PRODUCTION_NODE_LOCK)
        self.assertEqual(
            set(TARGETS),
            set(node_lock["packageTargets"]) | set(node_lock["licenseTargets"]),
        )
        for artifact in node_lock["artifacts"].values():
            self.assertEqual(64, len(artifact["sha256"]))
            self.assertIn(artifact["format"], ("tar.xz", "zip"))

        bubblewrap_lock = load_source_lock(PRODUCTION_BUBBLEWRAP_LOCK)
        self.assertEqual("0.11.2", bubblewrap_lock.version)
        self.assertEqual(64, len(bubblewrap_lock.archive_sha256))

    def test_node_lock_requires_an_explicit_musl_binary(self) -> None:
        lock = load_node_lock(PRODUCTION_NODE_LOCK)

        with self.assertRaisesRegex(RuntimeError, "pass --node-bin"):
            artifact_for_target(lock, "x86_64-unknown-linux-musl")
        license_artifact = artifact_for_target(
            lock,
            "x86_64-unknown-linux-musl",
            license_only=True,
        )
        self.assertEqual("linux-x64", license_artifact.key)

    def test_local_overrides_build_canonical_package(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            server_binary = executable_file(root / "zeta-source", b"zeta-server")
            daemon_binary = executable_file(root / "daemon-source", b"zeta-app-server-daemon")
            rg_binary = executable_file(root / "rg-source", b"ripgrep")
            output = root / "package"
            spec = TARGETS["aarch64-apple-darwin"]
            ripgrep = resolve_ripgrep(
                spec,
                PRODUCTION_LOCK,
                root / "cache",
                explicit_binary=rg_binary,
            )
            node = test_node_resolution(root, spec)

            build_package_directory(
                output,
                REPOSITORY_ROOT,
                read_workspace_version(REPOSITORY_ROOT / "Cargo.toml"),
                spec,
                server_binary,
                daemon_binary,
                ripgrep,
                node,
            )

            self.assertEqual(b"zeta-server", (output / "bin" / "zeta-server").read_bytes())
            self.assertEqual(
                b"zeta-app-server-daemon",
                (output / "bin" / "zeta-app-server-daemon").read_bytes(),
            )
            self.assertEqual(b"ripgrep", (output / "zeta-path" / "rg").read_bytes())
            self.assertEqual(
                b"node",
                (output / "zeta-resources" / "node" / "bin" / "node").read_bytes(),
            )
            self.assertTrue(os.access(str(output / "bin" / "zeta-server"), os.X_OK))
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
                (REPOSITORY_ROOT / "third_party" / "vscode" / "LICENSE.txt").read_text(encoding="utf-8"),
                (
                    output
                    / "zeta-resources"
                    / "licenses"
                    / "vscode"
                    / "LICENSE.txt"
                ).read_text(encoding="utf-8"),
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
            self.assertTrue(
                (output / "zeta-resources" / "extensions").is_dir()
            )
            product_services = json.loads(
                (
                    output
                    / "zeta-resources"
                    / "product-services"
                    / "product-services.json"
                ).read_text(encoding="utf-8")
            )
            self.assertEqual(
                "marketplace-root.json",
                product_services["marketplaceManager"]["trustedRoot"],
            )
            self.assertEqual(
                (
                    REPOSITORY_ROOT
                    / "resources"
                    / "product-services"
                    / "marketplace-root.json"
                ).read_bytes(),
                (
                    output
                    / "zeta-resources"
                    / "product-services"
                    / "marketplace-root.json"
                ).read_bytes(),
            )
            self.assertEqual(
                [
                    path.name
                    for path in sorted(
                        (output / "zeta-resources" / "extensions").iterdir(),
                        key=lambda path: path.name,
                    )
                ],
                self.BUILT_IN_EXTENSIONS,
            )
            self.assertIn(
                '"name": "json"',
                (
                    output
                    / "zeta-resources"
                    / "extensions"
                    / "json"
                    / "package.json"
                ).read_text(encoding="utf-8"),
            )

            self.assert_extension_resources(output / "zeta-resources" / "extensions")
            metadata = json.loads(
                (output / "zeta-package.json").read_text(encoding="utf-8")
            )
            self.assertEqual(2, metadata["layoutVersion"])
            self.assertEqual({"kind": "packagedNode"}, metadata["javascriptRuntime"])
            self.assertEqual("aarch64-apple-darwin", metadata["target"])
            self.assertEqual("local-override", metadata["components"]["ripgrep"]["source"])
            self.assertEqual("local-override", metadata["components"]["node"]["source"])
            self.assertEqual(
                hashlib.sha256(b"ripgrep").hexdigest(),
                metadata["components"]["ripgrep"]["binarySha256"],
            )
            self.assertEqual(
                hashlib.sha256(b"node").hexdigest(),
                metadata["components"]["node"]["binarySha256"],
            )

            with self.assertRaisesRegex(RuntimeError, "Refusing to replace"):
                build_package_directory(
                    output,
                    REPOSITORY_ROOT,
                    "0.1.0",
                    spec,
                    server_binary,
                    daemon_binary,
                    ripgrep,
                    node,
                )

    def test_host_provided_runtime_package_omits_standalone_node(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            spec = TARGETS["aarch64-apple-darwin"]
            output = root / "package"
            build_package_directory(
                output,
                REPOSITORY_ROOT,
                read_workspace_version(REPOSITORY_ROOT / "Cargo.toml"),
                spec,
                executable_file(root / "zeta-source", b"zeta-server"),
                executable_file(root / "daemon-source", b"zeta-app-server-daemon"),
                resolve_ripgrep(
                    spec,
                    PRODUCTION_LOCK,
                    root / "cache",
                    explicit_binary=executable_file(root / "rg-source", b"ripgrep"),
                ),
                None,
            )

            metadata = json.loads(
                (output / "zeta-package.json").read_text(encoding="utf-8")
            )
            self.assertEqual(
                {"kind": "hostProvidedNode"}, metadata["javascriptRuntime"]
            )
            self.assertNotIn("node", metadata["components"])
            self.assertFalse((output / "zeta-resources" / "node").exists())
            self.assertFalse(
                (output / "zeta-resources" / "licenses" / "node").exists()
            )

    def assert_extension_resources(self, extensions: Path) -> None:
        self.assertEqual(
            self.BUILT_IN_EXTENSIONS,
            sorted(path.name for path in extensions.iterdir() if path.is_dir()),
        )
        seen_ids = set()
        file_templates = []
        for package_name in self.BUILT_IN_EXTENSIONS:
            package = extensions / package_name
            manifest = json.loads((package / "package.json").read_text(encoding="utf-8"))
            extension_id = "{}.{}".format(manifest["publisher"], manifest["name"])
            self.assertNotIn(extension_id, seen_ids)
            seen_ids.add(extension_id)
            self.assertTrue(manifest["version"])
            contributes = manifest.get("contributes", {})
            for language in contributes.get("languages", []):
                if "configuration" in language:
                    self.assert_extension_resource(package, language["configuration"])
            for contribution_name in ("grammars", "snippets", "themes"):
                for contribution in contributes.get(contribution_name, []):
                    self.assert_extension_resource(package, contribution["path"])
                    if contribution_name == "snippets":
                        snippet_path = package.joinpath(
                            *contribution["path"][2:].split("/")
                        )
                        snippet_document = json.loads(
                            snippet_path.read_text(encoding="utf-8")
                        )
                        languages = contribution["language"]
                        if isinstance(languages, str):
                            languages = [languages]
                        for snippet_name, snippet in snippet_document.items():
                            if snippet.get("isFileTemplate") is True:
                                file_templates.extend(
                                    (extension_id, language, snippet_name)
                                    for language in languages
                                )
        self.assertEqual(
            [
                ("vscode.html", "html", "html doc"),
                ("vscode.javascript", "javascript", "Class Definition"),
                ("vscode.javascript", "javascriptreact", "Class Definition"),
                ("vscode.typescript", "typescript", "Class Definition"),
                ("vscode.typescript", "typescriptreact", "Class Definition"),
            ],
            sorted(file_templates),
        )

    def assert_extension_resource(self, package: Path, relative_path: str) -> None:
        self.assertTrue(relative_path.startswith("./"))
        normalized = relative_path[2:]
        self.assertNotIn("\\", normalized)
        self.assertNotIn("..", Path(normalized).parts)
        resource = package.joinpath(*normalized.split("/"))
        self.assertTrue(resource.is_file(), "missing extension resource: {}".format(resource))

    def test_repository_builtin_extension_contract(self) -> None:
        self.assert_extension_resources(REPOSITORY_ROOT / "extensions")

    def test_linux_package_contains_built_sandbox_resource_contract(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            source_archive = root / "bubblewrap-test.tar.xz"
            write_bubblewrap_source_archive(source_archive)
            bubblewrap_lock = write_bubblewrap_lock(root, source_archive)
            server_binary = executable_file(root / "zeta-source", b"zeta-server")
            daemon_binary = executable_file(root / "daemon-source", b"zeta-app-server-daemon")
            rg_binary = executable_file(root / "rg-source", b"ripgrep")
            bwrap_binary = executable_file(root / "bwrap-source", b"bubblewrap")
            spec = TARGETS["x86_64-unknown-linux-musl"]
            ripgrep = resolve_ripgrep(
                spec,
                PRODUCTION_LOCK,
                root / "rg-cache",
                explicit_binary=rg_binary,
            )
            node = test_node_resolution(root, spec)
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
                server_binary,
                daemon_binary,
                ripgrep,
                node,
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
            server_binary = root / "zeta-server.exe"
            server_binary.write_bytes(b"zeta-server")
            daemon_binary = root / "zeta-app-server-daemon.exe"
            daemon_binary.write_bytes(b"zeta-app-server-daemon")
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
            node = test_node_resolution(root, spec)
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
                server_binary,
                daemon_binary,
                ripgrep,
                node,
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

    def test_builtin_extension_copy_rejects_an_empty_source(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            source = root / "source"
            source.mkdir()

            with self.assertRaisesRegex(RuntimeError, "source is empty"):
                copy_builtin_extensions(source, root / "destination")

    def test_builtin_extension_copy_rejects_an_empty_package_directory(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            source = root / "source"
            (source / "demo").mkdir(parents=True)

            with self.assertRaisesRegex(RuntimeError, "missing package.json"):
                copy_builtin_extensions(source, root / "destination")

    @unittest.skipIf(os.name == "nt", "creating symbolic links may require Windows privilege")
    def test_builtin_extension_copy_rejects_a_symbolic_source_directory(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            target = root / "source-target"
            target.mkdir()
            source = root / "source"
            source.symlink_to(target, target_is_directory=True)

            with self.assertRaisesRegex(RuntimeError, "source is not a real directory"):
                copy_builtin_extensions(source, root / "destination")

    @unittest.skipIf(os.name == "nt", "creating symbolic links may require Windows privilege")
    def test_builtin_extension_copy_rejects_a_symbolic_package_directory(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            source = root / "source"
            source.mkdir()
            extension = root / "demo-target"
            extension.mkdir()
            (extension / "package.json").write_text(
                '{"name":"demo","publisher":"zeta","version":"1.0.0"}',
                encoding="utf-8",
            )
            (source / "demo").symlink_to(extension, target_is_directory=True)

            with self.assertRaisesRegex(RuntimeError, "Invalid built-in extension package"):
                copy_builtin_extensions(source, root / "destination")

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

    def test_node_fetches_and_extracts_exact_tar_members(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            archive = root / "node-test.tar.xz"
            write_node_tar_archive(archive, "bundle/bin/node", "bundle/LICENSE")
            lock_path = write_node_lock(
                root,
                archive,
                "tar.xz",
                "bundle/bin/node",
                "bundle/LICENSE",
                "x86_64-unknown-linux-gnu",
            )

            resolution = resolve_node(
                TARGETS["x86_64-unknown-linux-gnu"],
                lock_path,
                root / "cache",
            )

            self.assertEqual(b"node-runtime", resolution.executable.read_bytes())
            self.assertEqual(b"node-license", resolution.license_file.read_bytes())
            self.assertTrue(os.access(str(resolution.executable), os.X_OK))

    def test_node_fetches_and_extracts_exact_zip_members(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            archive = root / "node-test.zip"
            with zipfile.ZipFile(str(archive), "w") as output:
                output.writestr("bundle/node.exe", b"node-runtime")
                output.writestr("bundle/LICENSE", b"node-license")
            lock_path = write_node_lock(
                root,
                archive,
                "zip",
                "bundle/node.exe",
                "bundle/LICENSE",
                "x86_64-pc-windows-msvc",
            )

            resolution = resolve_node(
                TARGETS["x86_64-pc-windows-msvc"],
                lock_path,
                root / "cache",
            )

            self.assertEqual(b"node-runtime", resolution.executable.read_bytes())
            self.assertEqual(b"node-license", resolution.license_file.read_bytes())
            self.assertEqual("node.exe", resolution.executable.name)

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


def write_node_tar_archive(path: Path, executable_member: str, license_member: str) -> None:
    with tarfile.open(str(path), "w:xz") as archive:
        for member_name, contents, mode in (
            (executable_member, b"node-runtime", 0o755),
            (license_member, b"node-license", 0o644),
        ):
            member = tarfile.TarInfo(member_name)
            member.size = len(contents)
            member.mode = mode
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


def write_node_lock(
    root: Path,
    archive: Path,
    archive_format: str,
    executable_member: str,
    license_member: str,
    target: str,
) -> Path:
    artifact_key = "test-artifact"
    lock = {
        "schemaVersion": 1,
        "runtime": "node",
        "version": "test",
        "source": {"baseUrl": root.as_uri()},
        "packageTargets": {target: artifact_key},
        "licenseTargets": {},
        "artifacts": {
            artifact_key: {
                "archive": archive.name,
                "size": archive.stat().st_size,
                "sha256": hashlib.sha256(archive.read_bytes()).hexdigest(),
                "format": archive_format,
                "executable": executable_member,
                "license": license_member,
            }
        },
    }
    lock_path = root / "node-lock.json"
    lock_path.write_text(json.dumps(lock), encoding="utf-8")
    return lock_path


def test_node_resolution(root: Path, spec) -> NodeResolution:
    executable = executable_file(root / spec.node_name, b"node")
    license_file = root / "node-license"
    license_file.write_bytes(b"node license")
    return NodeResolution(
        executable=executable,
        license_file=license_file,
        version="24.18.1-test",
        source="local-override",
        binary_sha256=hashlib.sha256(b"node").hexdigest(),
        archive="node-test.zip",
        archive_sha256="a" * 64,
    )


if __name__ == "__main__":
    unittest.main()
