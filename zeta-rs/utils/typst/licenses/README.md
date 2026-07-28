# Typst third-party license texts

This directory contains license and notice texts for the upstream Typst
components and assets distributed through `zeta-typst`. It does not define the
license of Zeta or of the first-party `zeta-typst` wrapper; those remain governed
by the repository root `LICENSE`.

The files correspond to the Typst and `typst-assets` versions locked by
`zeta-rs/Cargo.lock`:

- `Typst.txt`: Typst's Apache-2.0 license text;
- `Typst-NOTICE.txt`: Typst's bundled third-party notices;
- `Typst-Assets-NOTICE.txt`: licenses and notices for bundled fonts and assets.

`desktop/licenses/` contains release-facing copies. When the Typst dependency,
features, fonts, or assets change, review the upstream license material and keep
the two locations byte-for-byte synchronized.
