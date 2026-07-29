# Bubblewrap source runtime

Zeta's Linux package builds an unprivileged Bubblewrap executable from the
official source archive locked by [`runtime-lock.json`](runtime-lock.json).
Bubblewrap is the Linux filesystem and network namespace enforcement mechanism;
`zeta-bwrap` remains the first-party typed argv builder and also exposes the
small Cargo binary wrapper around the upstream C entrypoint.

The package builder verifies the source archive size and SHA-256, extracts only
the locked regular-file members into `third_party/.cache/bubblewrap/`, sets
`ZETA_BWRAP_SOURCE_DIR`, and builds the `bwrap` Cargo binary. The C build needs
a target C compiler and `libcap` discoverable through `pkg-config`. Cross-build
jobs must provide the normal `PKG_CONFIG_*` sysroot settings.

Linux release packages place the resulting executable at
`zeta-resources/bwrap` and copy the upstream `COPYING` file to
`zeta-resources/licenses/bubblewrap/`. A prebuilt or signed helper may be
supplied with `--bwrap-bin`; source material is still verified so the package
contains the corresponding notices.

The bundled build intentionally does not enable setuid support. Runtime
selection validates the executable and probes the command-line capabilities
required by Zeta before accepting it.
