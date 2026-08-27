import hashlib
import json
import re
import sys
import tempfile
import unittest
from pathlib import Path


REPOSITORY_ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(REPOSITORY_ROOT / "build" / "release"))

from zeta_package.targets import TARGETS  # noqa: E402
from zeta_package.v8 import (  # noqa: E402
    DEFAULT_LOCK,
    LockedFile,
    load_v8_lock,
    materialize,
)
from zeta_package.v8 import resolve_v8_cargo_env  # noqa: E402


class V8ArtifactTests(unittest.TestCase):
    def test_lock_covers_every_release_target_and_build_system_pin(self) -> None:
        pairs = load_v8_lock()
        self.assertEqual(set(TARGETS), set(pairs))

        cargo_lock = (REPOSITORY_ROOT / "Cargo.lock").read_text()
        versions = set(
            re.findall(
                r'\[\[package\]\]\nname = "v8"\nversion = "([^"]+)"', cargo_lock
            )
        )
        self.assertEqual({next(iter(pairs.values())).version}, versions)

        module = (REPOSITORY_ROOT / "MODULE.bazel").read_text()
        target_build = (
            REPOSITORY_ROOT / "third_party" / "v8" / "BUILD.bazel"
        ).read_text()
        for target, pair in pairs.items():
            for artifact in (pair.archive, pair.binding):
                self.assertIn(artifact.name, module)
                self.assertIn(artifact.sha256, module)
            repository_fragment = target.replace("-", "_")
            self.assertIn(repository_fragment, target_build)
        self.assertEqual(8, module.count('["v8_enable_sandbox"]'))

    def test_materialize_replaces_a_corrupt_cached_file(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            source = root / "source.gz"
            source.write_bytes(b"verified-v8")
            digest = hashlib.sha256(source.read_bytes()).hexdigest()
            artifact = LockedFile("archive.gz", digest, source.as_uri())
            cache = root / "cache"
            cached = cache / artifact.name
            cache.mkdir()
            cached.write_bytes(b"corrupt")

            resolved = materialize(artifact, cache)

            self.assertEqual(b"verified-v8", resolved.read_bytes())

    def test_environment_overrides_are_pairwise(self) -> None:
        spec = TARGETS["aarch64-apple-darwin"]
        self.assertEqual(
            {},
            resolve_v8_cargo_env(
                spec,
                environ={
                    "RUSTY_V8_ARCHIVE": "/archive",
                    "RUSTY_V8_SRC_BINDING_PATH": "/binding",
                },
            ),
        )
        self.assertEqual(
            {}, resolve_v8_cargo_env(spec, environ={"V8_FROM_SOURCE": "true"})
        )
        with self.assertRaisesRegex(RuntimeError, "must be set together"):
            resolve_v8_cargo_env(spec, environ={"RUSTY_V8_ARCHIVE": "/archive"})

    def test_lock_rejects_a_target_gap(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary) / "runtime-lock.json"
            document = json.loads(DEFAULT_LOCK.read_text())
            del document["artifacts"]["aarch64-apple-darwin"]
            path.write_text(json.dumps(document))
            with self.assertRaisesRegex(RuntimeError, "target set is incomplete"):
                load_v8_lock(path)


if __name__ == "__main__":
    unittest.main()
