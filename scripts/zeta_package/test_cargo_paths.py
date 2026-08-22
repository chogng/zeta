"""Tests for shared Cargo output-path resolution."""

import unittest
from pathlib import Path

from zeta_package.cargo_paths import cargo_profile_directory
from zeta_package.cargo_paths import cargo_artifact_executable
from zeta_package.cargo_paths import cargo_rendered_diagnostic
from zeta_package.cargo_paths import parse_cargo_message
from zeta_package.cargo_paths import resolve_cargo_target_directory


class CargoPathsTests(unittest.TestCase):
    def test_uses_workspace_target_by_default(self) -> None:
        root = Path("/workspace/zeta")
        self.assertEqual(resolve_cargo_target_directory(root, {}), root / "target")

    def test_resolves_relative_override_from_workspace(self) -> None:
        root = Path("/workspace/zeta")
        self.assertEqual(
            resolve_cargo_target_directory(root, {"CARGO_TARGET_DIR": "build/cargo"}),
            root / "build" / "cargo",
        )

    def test_preserves_absolute_override(self) -> None:
        root = Path("/workspace/zeta")
        self.assertEqual(
            resolve_cargo_target_directory(root, {"CARGO_TARGET_DIR": "/cache/zeta"}),
            Path("/cache/zeta"),
        )

    def test_maps_only_the_builtin_dev_profile_directory(self) -> None:
        self.assertEqual(cargo_profile_directory("dev"), "debug")
        self.assertEqual(cargo_profile_directory("dev-small"), "dev-small")
        self.assertEqual(cargo_profile_directory("release"), "release")

    def test_reads_executables_and_diagnostics_from_cargo_messages(self) -> None:
        artifact = parse_cargo_message(
            '{"reason":"compiler-artifact","target":{"kind":["bin"],'
            '"name":"zeterm"},"executable":"/custom/target/zeterm"}'
        )
        self.assertEqual(
            cargo_artifact_executable(artifact, "zeterm"),
            "/custom/target/zeterm",
        )
        self.assertIsNone(cargo_artifact_executable(artifact, "other"))
        self.assertEqual(
            cargo_rendered_diagnostic(
                {
                    "reason": "compiler-message",
                    "message": {"rendered": "warning\n"},
                }
            ),
            "warning\n",
        )
        self.assertIsNone(parse_cargo_message("not JSON"))


if __name__ == "__main__":
    unittest.main()
