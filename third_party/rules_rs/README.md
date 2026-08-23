# `rules_rs` pin

The repository pins `rules_rs 0.0.96` through the archive override in the root
[`MODULE.bazel`](../../MODULE.bazel). The local patches are:

- `module_dot_bazel_version.patch`, which preserves module version metadata
  when the archive override bypasses the Bazel Central Registry patch set.
- `windows_gnullvm_exec_triples.patch`, which makes Windows Rust host tools use
  the same gnullvm ABI as the repository's hermetic LLVM/MinGW C++ toolchain.
  This is required for `rustc` to load proc-macro DLLs and link Bazel host tools
  without relying on an installed MSVC SDK. Because upstream places Cargo and
  rustc in separate repositories while gnullvm Cargo dynamically loads the
  matching `libunwind.dll`, the patch also merges the rustc runtime component
  into Cargo's repository. This keeps `cargo metadata` working when a manifest
  change reruns the crate extension. Remove the patch when `rules_rs` can select
  the Windows execution ABI and assemble its runtime DLLs from platform constraints.

The Cargo graph intentionally has one root workspace. `rules_rs` therefore sees
`zeterm`, its direct child crates, and `zeta-rs/*` in one `cargo metadata` result. Zeterm-owned
and shared crates resolve to the same `@crates` hub; no cross-workspace metadata
bridge or duplicate product hub is required.

Verify the integration from the repository root:

```bash
bazel query //zeterm:zeterm
bazel build //zeterm:zeterm_sources
bazel test //zeterm:zeterm_ci
```

If a newer rules_rs release is adopted, first remove the archive override in a
throwaway change and run the same graph checks. Keep a local patch only when an
upstream behavior is still required by this repository and document its exact
ownership here.
