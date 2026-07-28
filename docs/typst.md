# Typst document compilation

```yaml
status: current narrow integration
owner: zeta-rs/typst and zeta-rs/app-server
consumers:
  - desktop
lastUpdated: 2026-07-28
```

This document owns the cross-crate architecture and trust model. Compiler
implementation details are canonical in
[`zeta-rs/typst/README.md`](../zeta-rs/typst/README.md).

## Decision

Zeta embeds Typst 0.15.1 as Rust libraries instead of invoking a system
`typst` executable. The initial product capability converts an in-memory Typst
source string to a connection-owned PDF resource. This gives an agent-editable
text representation and a typeset paper output without granting the renderer
or compiler access to host paths.

This capability complements rather than replaces the Academic editor.
ProseMirror owns structured paper editing and agent-visible document state; a
deterministic serializer must translate that state into Typst source. Typst
then owns typesetting and PDF output. Monaco remains a Code product editor and
is not part of this paper pipeline.

## Ownership and end-to-end flow

| Component | Ownership |
| --- | --- |
| `zeta-typst` | compiler `World`, bundled fonts, source limits, diagnostics, PDF bytes |
| `zeta-app-server-protocol` | `document/typst/compile` DTOs and capability negotiation |
| `zeta-app-server` | request dispatch and connection-owned PDF resource creation |
| Desktop main/preload | exact IPC validation and typed capability bridge |
| Academic Workbench contribution | ProseMirror editor; future Typst serialization, diagnostics, preview, save/export |

Proposed Academic rendering flow (the serializer and preview are not yet
implemented):

```text
Academic ProseMirror document
-> deterministic Typst serializer
-> Typst source string
-> sandboxed preload API: typst.compile
-> trusted Electron main IPC route
-> document/typst/compile
-> zeta-typst in-memory World
-> PDF bytes
-> app-server ResourceStore
-> resource/read chunks
-> workbench PDF preview or explicit export
```

Compilation failures return `{ status: "failed", diagnostics }`. Successful
compilation returns `{ status: "success", resource, warnings }`; the resource
uses `application/pdf`, a 300-second TTL, a 16 MiB resource limit, connection
ownership, and the existing chunked Base64 read contract.

## Security and determinism

Current invariants:

- source is capped at 1 MiB measured as UTF-8 bytes;
- only the virtual `/main.typ` file exists;
- other project files and packages are denied;
- no network or Typst Universe package download occurs;
- no system fonts or arbitrary font files are loaded;
- current date access is unavailable;
- PDF bytes never receive a host path and remain connection-owned;
- Typst and related direct crates are exactly pinned to 0.15.1.

The renderer sandbox and this compiler boundary solve different problems.
Electron's sandbox limits renderer privileges. `InMemoryWorld` limits what
Typst source can request from the Rust host. Both boundaries remain necessary.

## Current status

Implemented:

- embedded Typst-to-PDF compilation with bundled fonts;
- typed app-server method, capability, diagnostics, and generated artifacts;
- desktop preload API plus resource metadata/read/release APIs;
- Academic ProseMirror editor pane and product-scoped registration;
- upstream Typst and bundled-font license notices;
- unit and integration tests for PDF output, diagnostics, ownership, and
  denied host-file access.

Current limitations:

- no multi-file projects, images, bibliographies, package imports, system
  fonts, incremental compilation, cancellation, or hard CPU deadline;
- no ProseMirror-to-Typst serializer, diagnostic projection, or PDF preview
  contribution;
- output is ephemeral until a caller explicitly reads and persists it;
- compilation currently runs synchronously and the method is declared global
  exclusive.

## Staged evolution

Near-term proposed work is the deterministic ProseMirror-to-Typst serializer,
followed by diagnostic projection into the structured document and preview of
the returned PDF.

Multi-file papers should later use a bounded immutable in-memory file map.
Bibliographies and images can be admitted only through explicit resource
types, aggregate byte/count limits, canonical virtual paths, and tests proving
that paths cannot escape the project root.

Package support, if required, needs a separate policy for version pinning,
download authority, cache integrity, notices, offline behavior, and malicious
WASM/plugin isolation. It is not part of the current capability.
