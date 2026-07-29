# `zeta-file-identity`

`zeta-file-identity` owns the platform boundary for reading stable file identity
and hard-link count from an already-open file. Consumers use
`FileInformation::from_file` when a trust decision must bind subsequent reads
to the same filesystem object, or `FileInformation::from_path` when inspecting
a controlled path before opening it.

The public contract intentionally exposes only `FileIdentity`,
`FileInformation`, and their comparison/link-count accessors. On Unix,
`platform::inspect` reads device, inode, and link count through
`MetadataExt`. On Windows it calls `GetFileInformationByHandle` through one
private FFI boundary and maps volume serial number plus file index to the same
domain-neutral identity. OS errors are returned unchanged as `io::Error`;
unsupported platforms fail explicitly.

```text
controlled path
└── FileInformation::from_path
    └── File::open
        └── FileInformation::from_file
            └── platform::inspect
                ├── Unix MetadataExt
                └── Windows GetFileInformationByHandle
```

This crate does not canonicalize paths, reject symlinks, decide whether hard
links are allowed, watch for mutations, or open files with domain-specific
flags. Those obligations remain with the caller. Adding path policy, Skill
semantics, or sandbox behavior here would signal architectural drift.

The Skills catalog is the current consumer: it rejects multi-link manifests
and verifies that the path still identifies the file handle whose contents were
scanned. Cross-crate ownership and trust semantics remain documented in
[`../../docs/skills.md`](../../docs/skills.md).

Current limitation: the crate supports the Unix and Windows host families used
by Zeta. Supporting another host requires a platform implementation that can
return both stable identity and link count; falling back to path spelling or
silently omitting link information is not compatible with this contract.

## Verification

```bash
cargo test --manifest-path zeta-rs/Cargo.toml -p zeta-file-identity
cargo clippy --manifest-path zeta-rs/Cargo.toml \
  -p zeta-file-identity --all-targets --no-deps -- -D warnings
```
