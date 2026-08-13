# Zeta package builder

This directory owns release staging for the canonical Zeta package layout. It
does not own runtime discovery, Tool policy, sandbox enforcement, signing,
notarization, installer formats, or update delivery.

```text
<package>/
├── zeta-package.json
├── bin/
│   └── zeta[.exe]
├── zeta-path/
│   └── rg[.exe]
└── zeta-resources/
    ├── bwrap                         # Linux only
    ├── zeta-command-runner.exe       # Windows only
    ├── zeta-windows-sandbox-setup.exe # Windows only
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
        ├── ripgrep/
        │   ├── LICENSE-MIT
        │   └── UNLICENSE
        └── vscode/LICENSE.txt        # built-in Editor Extension resources
```

The stable entry point is `scripts/build_zeta_package.py`. If `--zeta-bin` is
omitted, `cargo.py` builds `zeta-cli` for the selected target. `ripgrep.py`
maps the package target through `third_party/ripgrep/runtime-lock.json`,
validates archive size and SHA-256 on every use, extracts only the locked
member, and rejects non-regular archive members. For Linux, `bubblewrap.py`
verifies and extracts the locked upstream source, then builds the `zeta-bwrap`
binary with the target C compiler and `libcap`; `--bwrap-bin` accepts an already
built or signed helper. For Windows, `windows_helpers.py` builds both
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

Desktop development uses the same locks and canonical layout through the Node
assembler at `desktop/scripts/prepare-dev-package.mjs`. It builds the
first-party executables with Cargo's `dev` profile, verifies and extracts the
target-specific runtime archives, stages the result beside
`desktop/.tmp/zeta-package`, and replaces the previous development package only
after validation. It neither installs nor invokes Python. This Python package
remains the release builder and retains its refusal to replace an explicit
output directory.

```sh
python3 scripts/build_zeta_package.py \
  --target aarch64-apple-darwin \
  --package-dir /absolute/path/to/zeta-package
```

Release jobs that already built or signed binaries should use `--zeta-bin` and
optionally `--rg-bin`; those overrides are copied verbatim and their binary
digest is recorded in `zeta-package.json`. Linux jobs can likewise pass
`--bwrap-bin`. Signing and archive serialization must happen after this staging
step. Windows jobs can supply `--windows-command-runner-bin` and
`--windows-sandbox-setup-bin`; omitting either causes the missing first-party
helper to be built for the selected target.

| Target | Current sandbox package state |
| --- | --- |
| macOS | Native Seatbelt; no helper executable |
| Linux | `zeta-resources/bwrap` is required and validated |
| Windows | Both AppContainer helpers are required and validated |

Tests are offline and cover target-lock completeness, package layout, all thirteen built-in
Extension packages, their referenced resources, real file-template declarations, the packaged VS Code
license text, built-in Skill/Extension staging and link
rejection, product-service trust bundle staging, tar/zip member/source extraction, Linux and
Windows helper layouts, executable permissions, refusal to overwrite, and
digest failure cleanup:

```sh
python3 -m unittest discover -s scripts -p 'test_*.py'
```

The Node development assembler's target selection, locked ripgrep selection,
and atomic replacement behavior are covered by:

```sh
node --test desktop/scripts/prepare-dev-package.test.mjs
```
