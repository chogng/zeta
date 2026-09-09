import sys
import tempfile
import unittest
from pathlib import Path
from xml.etree import ElementTree

REPOSITORY_ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPOSITORY_ROOT))
sys.path.insert(0, str(Path(__file__).resolve().parent))

from windows_sandbox_runtime import runtime_definition
from windows_sandbox_runtime import stable_guid


class WindowsSandboxRuntimeTests(unittest.TestCase):
    def test_definition_installs_one_machine_wide_product_service(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            definition = runtime_definition(
                "1.2.3",
                root / "zeta-windows-sandbox-service.exe",
                root / "zeta-windows-sandbox-worker.exe",
                root / "zeta-command-runner.exe",
            )
        self.assertIn('Scope="perMachine"', definition)
        self.assertIn('Name="ZetaSandboxService"', definition)
        self.assertIn('Account="LocalSystem"', definition)
        self.assertIn('Start="install" Stop="both" Remove="uninstall"', definition)
        self.assertIn('Id="ProgramFiles64Folder"', definition)
        self.assertIn('Name="zeta-windows-sandbox-worker.exe"', definition)
        self.assertEqual(definition.count("<ServiceInstall "), 1)
        ElementTree.fromstring(definition)

    def test_upgrade_identity_is_stable_and_product_bound(self) -> None:
        self.assertEqual(
            stable_guid("Zeta.WindowsSandboxRuntime"),
            stable_guid("Zeta.WindowsSandboxRuntime"),
        )
        self.assertNotEqual(
            stable_guid("Zeta.WindowsSandboxRuntime"),
            stable_guid("Other.WindowsSandboxRuntime"),
        )


if __name__ == "__main__":
    unittest.main()
