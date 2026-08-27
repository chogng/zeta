"""Bazel module extension for pinned Zeta release package archives."""

_ZETA_RELEASE_BUILD_FILE = """\\
package(default_visibility = ["//visibility:public"])

filegroup(
    name = "zeta",
    srcs = [{entrypoint}],
)

filegroup(
    name = "package",
    srcs = glob([
        {manifest},
        {binaries},
        {resources},
        {path},
    ]),
)
"""


def _zeta_release_repository_impl(repository_ctx):
    repository_ctx.download_and_extract(
        sha256 = repository_ctx.attr.sha256,
        url = repository_ctx.attr.urls,
    )

    manifest_path = repository_ctx.attr.manifest
    manifest = json.decode(repository_ctx.read(manifest_path))
    entrypoint = manifest.get("entrypoint")
    path_directory = manifest.get("pathDir")
    resources_directory = manifest.get("resourcesDir")
    if not entrypoint or not path_directory or not resources_directory:
        fail("{} must declare entrypoint, pathDir, and resourcesDir".format(manifest_path))

    entrypoint_directory = entrypoint.rpartition("/")[0]
    repository_ctx.file(
        "BUILD.bazel",
        _ZETA_RELEASE_BUILD_FILE.format(
            binaries = json.encode(entrypoint_directory + "/**"),
            entrypoint = json.encode(entrypoint),
            manifest = json.encode(manifest_path),
            path = json.encode(path_directory + "/**"),
            resources = json.encode(resources_directory + "/**"),
        ),
    )
    return repository_ctx.repo_metadata(reproducible = True)


_zeta_release_repository = repository_rule(
    implementation = _zeta_release_repository_impl,
    attrs = {
        "manifest": attr.string(default = "zeta-package.json"),
        "sha256": attr.string(mandatory = True),
        "urls": attr.string_list(mandatory = True),
    },
)


_RELEASE = tag_class(
    attrs = {
        "manifest": attr.string(default = "zeta-package.json"),
        "sha256": attr.string(mandatory = True),
        "urls": attr.string_list(mandatory = True),
        "version": attr.string(mandatory = True),
    },
)


def _zeta_release_archive_impl(module_ctx):
    for module in module_ctx.modules:
        for release in module.tags.release:
            _zeta_release_repository(
                manifest = release.manifest,
                name = "zeta_release_{}_linux_x86_64".format(release.version),
                sha256 = release.sha256,
                urls = release.urls,
            )

    return module_ctx.extension_metadata(reproducible = True)


zeta_release_archive = module_extension(
    implementation = _zeta_release_archive_impl,
    tag_classes = {"release": _RELEASE},
)
