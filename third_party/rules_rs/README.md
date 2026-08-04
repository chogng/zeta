# `rules_rs` pin

The repository pins `rules_rs 0.0.96` through the archive override in the root
[`MODULE.bazel`](../../MODULE.bazel). The only local patch is
`module_dot_bazel_version.patch`, which preserves the module version metadata
when the archive override bypasses the Bazel Central Registry patch set.

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
