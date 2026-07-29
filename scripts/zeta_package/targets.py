"""Supported Zeta package targets and host target detection."""

import platform
from dataclasses import dataclass
from typing import Dict


@dataclass(frozen=True)
class TargetSpec:
    target: str
    is_windows: bool

    @property
    def is_linux(self) -> bool:
        return "linux" in self.target

    @property
    def executable_suffix(self) -> str:
        return ".exe" if self.is_windows else ""

    @property
    def zeta_name(self) -> str:
        return "zeta" + self.executable_suffix

    @property
    def ripgrep_name(self) -> str:
        return "rg" + self.executable_suffix


TARGETS: Dict[str, TargetSpec] = {
    target: TargetSpec(target=target, is_windows="windows" in target)
    for target in (
        "aarch64-apple-darwin",
        "aarch64-pc-windows-msvc",
        "aarch64-unknown-linux-gnu",
        "aarch64-unknown-linux-musl",
        "x86_64-apple-darwin",
        "x86_64-pc-windows-msvc",
        "x86_64-unknown-linux-gnu",
        "x86_64-unknown-linux-musl",
    )
}

HOST_TARGETS = {
    ("darwin", "aarch64"): "aarch64-apple-darwin",
    ("darwin", "x86_64"): "x86_64-apple-darwin",
    ("linux", "aarch64"): "aarch64-unknown-linux-gnu",
    ("linux", "x86_64"): "x86_64-unknown-linux-gnu",
    ("windows", "aarch64"): "aarch64-pc-windows-msvc",
    ("windows", "x86_64"): "x86_64-pc-windows-msvc",
}


def default_target() -> str:
    system = platform.system().lower()
    machine = normalize_machine(platform.machine())
    target = HOST_TARGETS.get((system, machine))
    if target is None:
        supported = ", ".join(sorted(TARGETS))
        raise RuntimeError(
            "Unsupported host platform {}/{}. Pass --target explicitly. "
            "Supported targets: {}".format(platform.system(), platform.machine(), supported)
        )
    return target


def normalize_machine(machine: str) -> str:
    normalized = machine.lower()
    if normalized in ("arm64", "aarch64"):
        return "aarch64"
    if normalized in ("amd64", "x86_64"):
        return "x86_64"
    return normalized
