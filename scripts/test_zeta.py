from __future__ import annotations

import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

import zeta


class ZetaLauncherTests(unittest.TestCase):
    def test_main_builds_once_and_runs_the_staged_zeta(self) -> None:
        staged = self._executables(Path("C:/staged"))
        built = self._executables(Path("C:/built"))
        rg = Path("C:/tools/rg.exe")
        with (
            patch.dict(zeta.os.environ, {"PATH": "tools"}, clear=True),
            patch.object(zeta, "build_binaries", return_value=(0, built)) as build,
            patch.object(zeta, "stage_runtime", return_value=staged),
            patch.object(zeta, "host_ripgrep", return_value=rg),
            patch.object(zeta.subprocess, "run") as run,
        ):
            run.return_value = zeta.subprocess.CompletedProcess([], 0)

            self.assertEqual(zeta.main(["--help"]), 0)

        build.assert_called_once_with(
            zeta.development_binaries(code_mode=None), {"PATH": "tools"}
        )
        runtime = zeta.runtime_environment({"PATH": "tools"}, staged, rg)
        run.assert_called_once_with(
            [str(staged["zeta"]), "--help"],
            cwd=zeta.REPOSITORY_ROOT,
            env=runtime,
            check=False,
        )

    def test_build_binaries_uses_one_cargo_invocation(self) -> None:
        binaries = ["zeta", "zeta-app-server-daemon"]
        with (
            tempfile.TemporaryDirectory() as temporary,
            patch.object(zeta, "built_executable") as built_executable,
            patch.object(zeta.subprocess, "run") as run,
        ):
            first = Path(temporary) / "zeta"
            second = Path(temporary) / "zeta-app-server-daemon"
            first.touch()
            second.touch()
            built_executable.side_effect = [first, second]
            run.return_value = zeta.subprocess.CompletedProcess([], 0)

            self.assertEqual(
                zeta.build_binaries(binaries, {"CARGO_BUILD_JOBS": "4"}),
                (0, {"zeta": first, "zeta-app-server-daemon": second}),
            )

        run.assert_called_once_with(
            [
                zeta.sys.executable,
                "-B",
                "scripts/cargo.py",
                "build",
                "--workspace",
                "--profile",
                zeta.DEVELOPMENT_PROFILE,
                "--bin",
                "zeta",
                "--bin",
                "zeta-app-server-daemon",
            ],
            cwd=zeta.REPOSITORY_ROOT,
            env={"CARGO_BUILD_JOBS": "4"},
            check=False,
        )

    def test_development_binaries_include_platform_children(self) -> None:
        self.assertIn(
            "zeta-command-runner",
            zeta.development_binaries(platform_name="win32"),
        )
        self.assertIn(
            "bwrap",
            zeta.development_binaries(platform_name="linux"),
        )
        self.assertIn(
            "zeta-code-mode-host",
            zeta.development_binaries(platform_name="darwin", code_mode="host"),
        )

    def test_stage_runtime_reuses_one_content_generation(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            first = root / "built/zeta.exe"
            second = root / "built/daemon.exe"
            first.parent.mkdir()
            first.write_bytes(b"zeta")
            second.write_bytes(b"daemon")
            with patch.object(zeta, "DEVELOPMENT_RUNTIME_ROOT", root / "runtime"):
                staged = zeta.stage_runtime({"zeta": first, "daemon": second})
                repeated = zeta.stage_runtime({"zeta": first, "daemon": second})

            self.assertEqual(staged, repeated)
            self.assertEqual(staged["zeta"].read_bytes(), b"zeta")
            self.assertEqual(staged["daemon"].read_bytes(), b"daemon")
            self.assertEqual(len(list((root / "runtime").iterdir())), 1)

    @staticmethod
    def _executables(root: Path) -> dict[str, Path]:
        return {
            "zeta": root / "zeta.exe",
            "zeta-app-server-daemon": root / "zeta-app-server-daemon.exe",
            "zeta-command-runner": root / "zeta-command-runner.exe",
            "zeta-windows-sandbox-setup": root / "zeta-windows-sandbox-setup.exe",
        }


if __name__ == "__main__":
    unittest.main()
