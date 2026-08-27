"""Rules for exposing one Zeta binary across supported release platforms."""

load("@rules_platform//platform_data:defs.bzl", "platform_data")

PLATFORMS = [
    "linux_arm64_musl",
    "linux_amd64_musl",
    "macos_amd64",
    "macos_arm64",
    "windows_amd64",
    "windows_arm64",
]

_PLATFORM_LABELS = {
    # The stock LLVM Windows platforms do not carry the ABI constraints that
    # rules_rs uses to select a Rust target triple. Use the explicit platforms
    # defined in the workspace root instead.
    "windows_amd64": "//:windows_x86_64_gnullvm",
    "windows_arm64": "//:windows_aarch64_gnullvm",
}


def multiplatform_binaries(name, platforms = PLATFORMS):
    """Creates manual platform transitions and one aggregate filegroup.

    The caller must define the binary named by `name` in the same package. The
    LLVM platform labels are kept in this helper so release BUILD files only
    describe the product target they publish.
    """
    for platform in platforms:
        platform_data(
            name = name + "_" + platform,
            platform = _PLATFORM_LABELS.get(
                platform,
                "@llvm//platforms:" + platform,
            ),
            target = name,
            tags = ["manual"],
        )

    native.filegroup(
        name = "release_binaries",
        srcs = [name + "_" + platform for platform in platforms],
        tags = ["manual"],
    )
