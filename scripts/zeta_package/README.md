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
    └── licenses/
        ├── bubblewrap/COPYING        # Linux only
        └── ripgrep/
            ├── LICENSE-MIT
            └── UNLICENSE
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
existing output directory.

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

Tests are offline and cover target-lock completeness, package layout, built-in
Skill staging/link rejection, tar/zip member/source extraction, Linux and
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
