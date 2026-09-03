# Zeta package builder

This directory owns release staging for the canonical Zeta package layout. It
does not own runtime discovery, Tool policy, sandbox enforcement, signing,
notarization, installer formats, or update delivery.

```text
<package>/
├── zeta-package.json
├── bin/
│   ├── zeta-app-server-daemon[.exe]
│   └── zeta-server[.exe]
├── zeta-path/
│   └── rg[.exe]
└── zeta-resources/
    ├── bwrap                         # Linux only
    ├── zeta-command-runner.exe       # Windows only
    ├── zeta-windows-sandbox-setup.exe # Windows only
    ├── node/                           # packaged-node variant only
    │   └── bin/
    │       └── node[.exe]          # shared JavaScript LSP runtime
    ├── skills/
    │   └── skill-creator/SKILL.md
    ├── extensions/
    │   ├── css/package.json
    │   ├── ...
    │   └── yaml/package.json
    ├── product-services/
    │   ├── product-services.json      # official service endpoints
    │   └── marketplace-root.json      # public pinned TUF root
    └── licenses/
        ├── bubblewrap/COPYING        # Linux only
        ├── node/LICENSE                # packaged-node variant only
        ├── ripgrep/
        │   ├── LICENSE-MIT
        │   └── UNLICENSE
        └── vscode/LICENSE.txt        # built-in Editor Extension resources
```

The stable entry point is `build/release/build_zeta_package.py`. Before resolving product binaries, it runs the App Server protocol generator into a temporary directory and binds the current protocol major, revision, and schema hash into `zeta-package.json`; it does not rewrite checked-in fixtures. `verify:protocol` remains an explicit fixture check, while `generate:protocol` refreshes repository fixtures when they are intentionally being reviewed. If `--server-bin` or
`--app-server-daemon-bin` is omitted, `cargo.py` builds the corresponding product-neutral
`zeta-server-host` or profile-scoped `zeta-app-server-daemon` for the selected target. `ripgrep.py`
maps the package target through `third_party/ripgrep/runtime-lock.json`,
validates archive size and SHA-256 on every use, extracts only the locked
member, and rejects non-regular archive members. `node.py` applies the same
locked size/SHA-256 gate to the shared Node.js runtime, extracts only `node[.exe]`
and its license, and never resolves Node from the host `PATH`. Official Node.js
releases do not contain musl builds, so musl release jobs must supply an exact
`--node-bin`; the lock still supplies the verified upstream license. For Linux, `bubblewrap.py` validates [`zeta-rs/vendor/bubblewrap`](../../zeta-rs/vendor/bubblewrap/README.md), then builds the `zeta-bwrap` binary with the target C compiler and `libcap`; `--bwrap-bin` accepts an already built or signed helper. For Windows, `windows_helpers.py` builds both
first-party AppContainer helpers
from `zeta-windows-sandbox`, or validates the two explicit helper overrides.
Repository-owned built-in Skills come from
`zeta-rs/skills/assets/`; `layout.py` rejects linked or malformed Skill trees,
stages them under `zeta-resources/skills/`, validates the complete package in a
sibling temporary directory, and renames it into place. It never replaces an
existing output directory. Repository-owned declarative Editor Extensions come from the root
`extensions/` directory and are copied to `zeta-resources/extensions/` with the same regular,
unlinked-tree restriction. Their canonical upstream license copy is
`third_party/vscode/LICENSE.txt` (mirrored from the sibling VS Code source checkout) and is copied
once to `zeta-resources/licenses/vscode/LICENSE.txt`. Runtime discovery and contribution semantics remain owned by
[`zeta-extensions`](../../zeta-rs/extensions/README.md) and
[`docs/editor-extensions.md`](../../docs/editor-extensions.md), not by the package builder.
Product service inputs come from `resources/product-services/`; both assemblers copy the regular
tree and require the official Marketplace config and pinned root before completing a package. The
runtime parser remains the trust authority for URLs, relative root containment, schema, and TUF
verification.

`--javascript-runtime packaged-node` is the default and retains standalone Node
for CLI, browser-bridge, remote, and headless App Server hosts.
`--javascript-runtime host-provided-node` omits the executable, license, and Node
component metadata; this variant is valid only when the product host injects an
exact Node-compatible executable. Electron Desktop uses that variant, declares
its exact `process.execPath` to the Rust App Server, and enters run-as-Node mode
only for JavaScript language-server children. Both alternatives are explicit in
package layout version 2 under `javascriptRuntime.kind`; validators reject a
payload whose files and declared runtime kind disagree.

Desktop development uses the same locks and canonical layout through the Node
assembler at `build/zeta-package/prepareDevPackage.ts`. It defaults to the
host-provided runtime variant for Electron; Browser full mode passes
`--javascript-runtime packaged-node`. The assembler builds first-party
executables with Cargo's compact `dev-small` profile, verifies and extracts the required
target-specific runtime archives, stages the result beside
`.build/zeta-package/dev/store-v1/<target>/<javascript-runtime>/dev-small/packages/<version>/<build-id>`, then publishes an immutable numbered manifest only
after full-file validation. The package store retains the selected and rollback packages and removes older packages only when no process lease is held. Host executables honor `CARGO_TARGET_DIR`, and the assembler
reads the exact executable path from Cargo's JSON artifact messages instead of
guessing a `target` layout. Normal compact host builds, the development
assembler, and the Rust watcher therefore reuse one compilation cache without
creating a second target-triple tree. It neither installs nor invokes Python.
This Python package remains the release builder, also honors `CARGO_TARGET_DIR`,
and retains its refusal to replace an explicit output directory.

```sh
python3 -B build/release/build_zeta_package.py \
  --target aarch64-apple-darwin \
  --package-dir /absolute/path/to/zeta-package
```

For an Electron-owned package payload:

```sh
python3 -B build/release/build_zeta_package.py \
  --target aarch64-apple-darwin \
  --javascript-runtime host-provided-node \
  --package-dir dist/zeta-electron
```

Release jobs that already built or signed binaries should use `--server-bin` and
`--app-server-daemon-bin`, and
optionally `--rg-bin` or, for the `packaged-node` variant, `--node-bin`; those overrides are copied verbatim and their binary
digest is recorded in `zeta-package.json`. `buildId` covers the sorted digest manifest of every package file together with all identity metadata except `buildId` and the file manifest itself; it is not a mutable release selector. Linux jobs can likewise pass
`--bwrap-bin`. Signing and archive serialization must happen after this staging
step. Windows jobs can supply `--windows-command-runner-bin` and
`--windows-sandbox-setup-bin`; omitting either causes the missing first-party
helper to be built for the selected target.

For app Remote delivery, one or more completed packaged-node directories can be serialized into
deterministic rootless archives and a strict local catalog:

```sh
python3 -B build/release/build_remote_runtime_bundle.py \
  --bundle-dir /absolute/path/to/remote-runtimes \
  --package-dir /absolute/path/to/x86_64-linux-package \
  --package-dir /absolute/path/to/aarch64-linux-package
```

The bundle builder rejects links, special files, duplicate targets, non-POSIX targets and package
metadata that does not match the canonical Remote contract. It records compressed size, unpacked
size and SHA-256 for each deterministic archive. Product packaging, not this canonical package
builder, authenticates the resulting catalog.

| Target | Current sandbox package state |
| --- | --- |
| macOS | Native Seatbelt; no helper executable |
| Linux | `zeta-resources/bwrap` is required and validated |
| Windows | Both AppContainer helpers are required and validated |

Tests are offline and cover target-lock completeness, both runtime package layouts, the packaged
Node executable/license and host-provided omission, all thirteen built-in
Extension packages, their referenced resources, real file-template declarations, the packaged VS Code
license text, built-in Skill/Extension staging and link
rejection, product-service trust bundle staging, tar/zip member/source extraction, Linux and
Windows helper layouts, executable permissions, refusal to overwrite, and
digest failure cleanup:

```sh
python3 -B scripts/test-python.py
```

The Node development assembler's target selection, locked ripgrep/Node selection,
and atomic replacement behavior are covered by:

```sh
node --test build/zeta-package/prepareDevPackage.test.ts
```
