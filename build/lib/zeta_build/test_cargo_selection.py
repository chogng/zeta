import subprocess
import sys
import unittest
from pathlib import Path
from unittest.mock import patch

REPOSITORY_ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(REPOSITORY_ROOT))

from build.lib.zeta_build.cargo_selection import cargo_command_uses_v8  # noqa: E402
from build.lib.zeta_build.cargo_selection import (  # noqa: E402
    cargo_tree_selection_arguments,
)


class CargoSelectionTests(unittest.TestCase):
    def test_forwards_only_dependency_selection_arguments(self) -> None:
        self.assertEqual(
            [
                "-p",
                "zeta-protocol",
                "--features=fixtures",
                "--target",
                "aarch64-apple-darwin",
                "--locked",
            ],
            cargo_tree_selection_arguments(
                [
                    "-p",
                    "zeta-protocol",
                    "--features=fixtures",
                    "--target",
                    "aarch64-apple-darwin",
                    "--locked",
                    "--lib",
                    "contract_tests",
                    "--",
                    "--exact",
                ]
            ),
        )

    @patch("build.lib.zeta_build.cargo_selection.subprocess.run")
    def test_skips_v8_for_an_unrelated_selected_package(self, run) -> None:
        run.return_value = subprocess.CompletedProcess(
            args=[], returncode=0, stdout="zeta-protocol v0.0.0\nserde v1.0.0\n"
        )

        self.assertFalse(
            cargo_command_uses_v8(
                "cargo", ["test", "-p", "zeta-protocol"], Path("/repository")
            )
        )
        self.assertIn("normal,build,dev", run.call_args.args[0])
        self.assertEqual(Path("/repository"), run.call_args.kwargs["cwd"])

    @patch("build.lib.zeta_build.cargo_selection.subprocess.run")
    def test_configures_v8_for_a_transitive_dependency(self, run) -> None:
        run.return_value = subprocess.CompletedProcess(
            args=[],
            returncode=0,
            stdout="app v0.0.0\nzeta-code-mode-runtime v0.0.0\nv8 v150.4.0\n",
        )

        self.assertTrue(
            cargo_command_uses_v8(
                "cargo", ["check", "--package=app"], Path("/repository")
            )
        )

    @patch("build.lib.zeta_build.cargo_selection.subprocess.run")
    def test_non_dependency_command_does_not_inspect_the_graph(self, run) -> None:
        self.assertFalse(
            cargo_command_uses_v8(
                "cargo", ["clean", "-p", "zeta-protocol"], Path("/repository")
            )
        )
        run.assert_not_called()


if __name__ == "__main__":
    unittest.main()
