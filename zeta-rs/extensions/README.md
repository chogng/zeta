# `zeta-extensions`

`zeta-extensions` owns the reusable domain contract for static extension packages. It is designed
for both the Electron App Server path and the future `zeterm` host.

## Boundary

The crate receives trusted `ExtensionRoot` values and owns `ExtensionCatalog::list` and
`ExtensionCatalog::open_resource`. `scan_root`, `discover_package`, `validate_relative_path`, and
`is_within` carry discovery, manifest identity validation, containment checks, and failure
semantics. It returns domain descriptors, diagnostics, and bounded bytes; it does not know JSON-RPC,
connection-owned resources, Workbench, TextMate, or extension code execution.

`AppServer` converts these values to protocol DTOs and places resource bytes in its connection-owned
`ResourceStore`. `zeterm` can depend on this crate directly without importing App Server or desktop
transport code.

## Package contract

Each direct child of a trusted root is a package containing `package.json`. The required identity
fields are `name`, `publisher`, and `version`; the canonical ID is `publisher.name`. Package and
resource paths are canonicalized and must remain below their trusted root/package. Manifest bytes
are limited to 4 MiB and resource bytes to 16 MiB.

The crate only validates the package envelope and exposes the complete manifest JSON. Product hosts
interpret `contributes` fields such as TextMate grammars, language configuration, snippets, or
themes. This keeps editor-specific semantics out of the shared Rust crate.

## Failure behavior and tests

Missing optional roots produce an empty catalog. Readable roots with invalid packages produce
structured diagnostics without registering those packages. Unsafe or missing resource requests
return typed errors; no filesystem fallback is attempted. `catalog_tests.rs` covers discovery,
resource reads, malformed manifests, and traversal rejection.
