# `zeta-typst`

> This README owns implementation details for the in-memory compiler boundary.
> Cross-process ownership, product semantics, and staged evolution are
> canonical in [`docs/typst.md`](../../../docs/typst.md).

`zeta-typst` compiles one caller-provided Typst source string to PDF. It owns
the Typst `World` implementation and deliberately provides no operating-system
filesystem, package registry, network, environment, or clock access.

## Boundary and public contract

`TypstCompiler` caches Typst's standard library, bundled fonts, and font book.
`TypstCompiler::compile` accepts UTF-8 source by reference and returns:

- `TypstCompileOutcome::Success` with PDF bytes and non-fatal diagnostics;
- `TypstCompileOutcome::Failed` for normal source or PDF-generation
  diagnostics;
- `TypstCompileError::SourceTooLarge` when source exceeds
  `MAX_TYPST_SOURCE_BYTES` (1 MiB, measured as UTF-8 bytes).

Source errors are outcomes rather than infrastructure errors so UI clients can
display them without parsing internal error strings. Ranges are half-open UTF-8
byte offsets into the exact submitted source.

The crate does not own RPC DTOs, PDF resource retention, editor models,
preview UI, or document persistence.

## Internal ownership and call path

```text
TypstCompiler::compile
|- size validation
|- InMemoryWorld::new(source)
|- typst::compile
|  `- World::{source,file,font,today}
|- map_diagnostics
`- typst_pdf::pdf
```

Key private symbols:

- `InMemoryWorld` binds `/main.typ` to the submitted source. `source` and
  `file` return `FileError::AccessDenied` for every other `FileId`; changing
  this is a trust-boundary change and requires updating tests and
  `docs/typst.md`.
- `map_diagnostics` removes Typst-internal types and translates spans through
  `WorldExt::range`. Moving this conversion into desktop code would leak the
  compiler dependency across the process boundary.
- `TypstCompiler::{library,book,fonts}` own immutable reusable compiler state.
  Per-document mutable state belongs to `InMemoryWorld`.

`today` always returns `None`. This keeps compilation independent of the host
clock; Typst documents that request the current date receive a source
diagnostic.

## Fonts and licensing

The `typst-assets` `fonts` feature supplies the current fixed font set. The
`zeta-typst` crate itself remains proprietary under the repository root
`LICENSE`; the upstream Apache license does not relicense this wrapper.

The exact upstream texts owned by this integration live in `licenses/`:

- `Typst.txt` is the Apache-2.0 license used by Typst;
- `Typst-NOTICE.txt` preserves Typst's required third-party attributions;
- `Typst-Assets-NOTICE.txt` preserves the licenses and attributions for the
  bundled fonts and other `typst-assets` material.

The desktop release must also ship `desktop/THIRD_PARTY_NOTICES.md` and the
matching files under `desktop/licenses/`. Those files are release-facing copies
of the component-local texts and must remain byte-for-byte synchronized.
Changing the Typst version, asset feature, or font source requires a license
review, synchronization of both locations, and a deterministic-output review.

## Tests and modification impact

Run:

```text
cargo test -p zeta-typst
```

Tests cover PDF output, ranged diagnostics, denied host-file access, and the
source byte limit. Changes to public result types also require protocol
fixture regeneration and app-server/desktop tests. Changes to file/package
access require a threat-model update before implementation.

## Current limitations and extension point

Current implementation accepts only `/main.typ`. It does not yet support
images, bibliography files, multi-file projects, Typst Universe packages,
system fonts, cancellation, execution time limits, or incremental
compilation.

The intended next extension is an immutable, size-bounded in-memory project
file map supplied by the app-server. It must not be implemented by accepting a
host root path. Cancellation and worker/process isolation must be designed
before enabling untrusted long-running workloads.
