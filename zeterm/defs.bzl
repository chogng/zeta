"""Bazel definitions for the zeterm application workspace."""

load("@crates//:defs.bzl", "all_crate_deps")
load("@rules_rust//rust:defs.bzl", "rust_binary", "rust_library")

def zeterm_rust_library(name, crate_name, package_name, data = []):
    """Defines a zeterm-owned Cargo library with metadata-derived dependencies.

    `package_name` is the Cargo workspace package path used by rules_rs. Keeping
    it explicit makes the Bazel target independent of the BUILD file directory.
    """
    rust_library(
        name = name,
        crate_name = crate_name,
        compile_data = data,
        deps = all_crate_deps(package_name = package_name),
        edition = "2024",
        srcs = native.glob(
            ["src/**/*.rs"],
            exclude = ["src/main.rs"],
        ),
        visibility = ["//visibility:public"],
    )

    native.filegroup(
        name = name + "_sources",
        srcs = native.glob([
            "src/**/*.rs",
            "Cargo.toml",
        ]),
        visibility = ["//visibility:public"],
    )

def zeterm_rust_binary(name, crate_name, package_name, crate_root, deps = [], data = []):
    """Defines the zeterm binary from the root workspace dependency graph."""
    rust_binary(
        name = name,
        crate_name = crate_name,
        compile_data = data,
        crate_root = crate_root,
        deps = all_crate_deps(package_name = package_name) + deps,
        edition = "2024",
        srcs = native.glob(["src/**/*.rs"]),
        visibility = ["//visibility:public"],
    )
