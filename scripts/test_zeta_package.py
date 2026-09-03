from __future__ import annotations

import json
import tempfile
import unittest
from pathlib import Path
from unittest.mock import call
from unittest.mock import patch

import zeta_package


class ZetaPackageLauncherTests(unittest.TestCase):
    def test_main_packages_then_runs_the_staged_cli_against_that_package(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            package = Path(temporary) / "package"
            suffix = ".exe" if zeta_package.os.name == "nt" else ""
            daemon = package / "bin" / f"zeta-app-server-daemon{suffix}"
            product_services = (
                package / "zeta-resources/product-services/product-services.json"
            )
            daemon.parent.mkdir(parents=True)
            product_services.parent.mkdir(parents=True)
            daemon.touch()
            product_services.touch()
            built = Path(temporary) / "built-zeta"
            staged = Path(temporary) / "staged-zeta"
            environment = {"PATH": "tools"}
            with (
                patch.dict(zeta_package.os.environ, environment, clear=True),
                patch.object(zeta_package, "current_package", return_value=package),
                patch.object(
                    zeta_package.zeta,
                    "build_binaries",
                    return_value=(0, {"zeta": built}),
                ) as build,
                patch.object(
                    zeta_package.zeta,
                    "stage_runtime",
                    return_value={"zeta": staged},
                ),
                patch.object(zeta_package.subprocess, "run") as run,
            ):
                run.side_effect = [
                    zeta_package.subprocess.CompletedProcess([], 0),
                    zeta_package.subprocess.CompletedProcess([], 0),
                ]

                self.assertEqual(
                    zeta_package.main(["app-server", "daemon", "version"]), 0
                )

            build.assert_called_once_with(["zeta"], environment)
            run.assert_has_calls(
                [
                    call(
                        ["node", "build/zeta-package/prepareDevPackage.ts"],
                        cwd=zeta_package.zeta.REPOSITORY_ROOT,
                        env=environment,
                        check=False,
                    ),
                    call(
                        [str(staged), "app-server", "daemon", "version"],
                        cwd=zeta_package.zeta.REPOSITORY_ROOT,
                        env={
                            **environment,
                            "ZETA_APP_SERVER_DAEMON_PATH": str(daemon.resolve()),
                            "ZETA_PRODUCT_SERVICES_PATH": str(
                                product_services.resolve()
                            ),
                        },
                        check=False,
                    ),
                ]
            )

    def test_current_package_resolves_the_latest_numbered_manifest(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            development_root = Path(temporary)
            build = "a" * 64
            package_root = development_root / "packages" / "0.1.0" / build
            package_root.mkdir(parents=True)
            manifests = development_root / "manifests"
            manifests.mkdir()
            (manifests / "00000000000000000001.json").write_text(
                json.dumps(
                    {
                        "formatVersion": 1,
                        "sequence": 1,
                        "directory": f"packages/0.1.0/{build}",
                    }
                ),
                encoding="utf-8",
            )

            with patch.object(
                zeta_package, "development_root", return_value=development_root
            ):
                self.assertEqual(zeta_package.current_package(), package_root)

    def test_current_package_rejects_a_path_outside_the_package_store(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            development_root = Path(temporary)
            manifests = development_root / "manifests"
            manifests.mkdir()
            (manifests / "00000000000000000001.json").write_text(
                json.dumps(
                    {"formatVersion": 1, "sequence": 1, "directory": "../package"}
                ),
                encoding="utf-8",
            )

            with patch.object(
                zeta_package, "development_root", return_value=development_root
            ):
                with self.assertRaisesRegex(
                    RuntimeError, "invalid Zeta development package manifest"
                ):
                    zeta_package.current_package()


if __name__ == "__main__":
    unittest.main()
