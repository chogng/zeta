"""Machine-wide MSI definition for the product-bound Zeta Windows sandbox service."""

import hashlib
import html
import json
import subprocess
from dataclasses import dataclass
from pathlib import Path

from build.lib.zeta_build.targets import TargetSpec
from zeta_package.layout import validate_package_directory
from zeta_package.windows_helpers import COMMAND_RUNNER_NAME
from zeta_package.windows_helpers import SANDBOX_SERVICE_NAME
from zeta_package.windows_helpers import SANDBOX_WORKER_NAME


SERVICE_NAME = "ZetaSandboxService"
INSTALL_DIRECTORY_NAME = "Sandbox"


@dataclass(frozen=True)
class WindowsSandboxRuntimePlan:
    definition: Path
    artifact: Path
    command: tuple[str, ...]


def prepare_runtime_msi(
    package: Path,
    spec: TargetSpec,
    output_directory: Path,
    wix: str = "wix",
) -> WindowsSandboxRuntimePlan:
    if not spec.is_windows:
        raise RuntimeError("The Zeta Windows sandbox runtime requires a Windows target")
    package = package.expanduser().resolve()
    validate_package_directory(package, spec)
    metadata = json.loads((package / "zeta-package.json").read_text(encoding="utf-8"))
    version = metadata.get("version")
    if not isinstance(version, str) or not version:
        raise RuntimeError("Zeta package metadata has no runtime version")
    resources = package / "zeta-resources"
    service = resources / SANDBOX_SERVICE_NAME
    runner = resources / COMMAND_RUNNER_NAME
    worker = resources / SANDBOX_WORKER_NAME
    output_directory = output_directory.expanduser().resolve()
    output_directory.mkdir(parents=True, exist_ok=True)
    stem = f"zeta-windows-runtime-{version}-{spec.architecture.value}"
    definition = output_directory / f"{stem}.wxs"
    artifact = output_directory / f"{stem}.msi"
    for output in (definition, artifact):
        if output.exists():
            raise RuntimeError(f"Refusing to replace existing Windows runtime output: {output}")
    definition.write_text(
        runtime_definition(version, service, worker, runner),
        encoding="utf-8",
    )
    architecture = "x64" if spec.architecture.value == "x86_64" else "arm64"
    return WindowsSandboxRuntimePlan(
        definition=definition,
        artifact=artifact,
        command=(
            wix,
            "build",
            str(definition),
            "-arch",
            architecture,
            "-o",
            str(artifact),
        ),
    )


def build_runtime_msi(plan: WindowsSandboxRuntimePlan) -> Path:
    if plan.artifact.exists():
        raise RuntimeError(
            f"Refusing to replace existing Windows runtime MSI: {plan.artifact}"
        )
    subprocess.run(plan.command, check=True)
    if not plan.artifact.is_file():
        raise RuntimeError("WiX succeeded without creating the Windows runtime MSI")
    return plan.artifact


def runtime_definition(
    version: str,
    service: Path,
    worker: Path,
    runner: Path,
) -> str:
    upgrade_code = stable_guid("Zeta.WindowsSandboxRuntime")
    values = {
        "version": html.escape(version, quote=True),
        "upgrade_code": upgrade_code,
        "service": html.escape(str(service), quote=True),
        "worker": html.escape(str(worker), quote=True),
        "runner": html.escape(str(runner), quote=True),
    }
    return (
        '<?xml version="1.0" encoding="UTF-8"?>'
        '<Wix xmlns="http://wixtoolset.org/schemas/v4/wxs">'
        '<Package Name="Zeta Windows Sandbox Runtime" Manufacturer="Zeta" '
        f'Version="{values["version"]}" UpgradeCode="{values["upgrade_code"]}" '
        'Scope="perMachine">'
        '<MajorUpgrade DowngradeErrorMessage="A newer Zeta Windows Sandbox Runtime is already installed."/>'
        '<MediaTemplate EmbedCab="yes"/>'
        '<StandardDirectory Id="ProgramFiles64Folder">'
        '<Directory Id="ZetaDirectory" Name="Zeta">'
        f'<Directory Id="INSTALLFOLDER" Name="{INSTALL_DIRECTORY_NAME}">'
        '<Component Id="SandboxServiceComponent" Guid="*">'
        f'<File Id="SandboxServiceFile" Name="{SANDBOX_SERVICE_NAME}" Source="{values["service"]}" KeyPath="yes"/>'
        f'<ServiceInstall Id="SandboxServiceInstall" Name="{SERVICE_NAME}" '
        'DisplayName="Zeta Windows Sandbox Service" Type="ownProcess" Start="auto" '
        'ErrorControl="normal" Account="LocalSystem" Arguments="--service"/>'
        f'<ServiceControl Id="SandboxServiceControl" Name="{SERVICE_NAME}" '
        'Start="install" Stop="both" Remove="uninstall" Wait="yes"/>'
        '</Component>'
        '<Component Id="SandboxClientComponent" Guid="*">'
        f'<File Id="CommandRunnerFile" Name="{COMMAND_RUNNER_NAME}" Source="{values["runner"]}" KeyPath="yes"/>'
        f'<File Id="SandboxWorkerFile" Name="{SANDBOX_WORKER_NAME}" Source="{values["worker"]}"/>'
        '</Component>'
        '</Directory></Directory></StandardDirectory>'
        '<Feature Id="Main" Title="Zeta Windows Sandbox Runtime" Level="1">'
        '<ComponentRef Id="SandboxServiceComponent"/>'
        '<ComponentRef Id="SandboxClientComponent"/>'
        '</Feature></Package></Wix>'
    )


def stable_guid(identity: str) -> str:
    value = bytearray(hashlib.sha256(identity.encode("utf-8")).digest()[:16])
    value[6] = (value[6] & 0x0F) | 0x50
    value[8] = (value[8] & 0x3F) | 0x80
    return (
        f"{{{value[0]:02X}{value[1]:02X}{value[2]:02X}{value[3]:02X}-"
        f"{value[4]:02X}{value[5]:02X}-{value[6]:02X}{value[7]:02X}-"
        f"{value[8]:02X}{value[9]:02X}-"
        f"{value[10]:02X}{value[11]:02X}{value[12]:02X}{value[13]:02X}{value[14]:02X}{value[15]:02X}}}"
    )
