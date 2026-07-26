"""Shared Bazel definitions for Rust crates in the Zeta workspace."""

load("@crates//:defs.bzl", "all_crate_deps")
load("@rules_rust//rust:defs.bzl", "rust_binary", "rust_library", "rust_test")

def zeta_rust_crate(name, crate_name, data = []):
    """Defines a Cargo library crate and its unit-test target.

    The crate's dependencies come from the workspace Cargo.lock through the
    generated `@crates` repository, so Bazel resolves the same dependency graph
    as Cargo. Each package must invoke this macro from its own BUILD file.
    """
    srcs = native.glob(
        ["src/**/*.rs"],
        exclude = ["src/main.rs"],
    )

    rust_library(
        name = name,
        crate_name = crate_name,
        compile_data = data,
        deps = all_crate_deps(),
        edition = "2024",
        srcs = srcs,
        visibility = ["//visibility:public"],
    )

    rust_test(
        name = name + "-unit-tests",
        crate = ":" + name,
        data = data,
        deps = all_crate_deps(
            normal = True,
            normal_dev = True,
        ),
    )

def zeta_rust_binary(name, crate_name, crate_root, deps):
    """Defines a Cargo binary using the lockfile-derived dependency graph.

    Callers provide the package library in `deps` when the binary imports it.
    This keeps the dependency edge explicit and mirrors Cargo's package layout.
    """
    rust_binary(
        name = name,
        crate_name = crate_name,
        crate_root = crate_root,
        deps = all_crate_deps() + deps,
        edition = "2024",
        srcs = native.glob(["src/**/*.rs"]),
        visibility = ["//visibility:public"],
    )
