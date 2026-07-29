# Third-party runtime artifacts

This directory stores the **source-controlled metadata** for native runtimes. It
does not store downloaded archives or platform binaries. CI and release builds
must fetch each locked artifact, verify its SHA-256 digest, and copy it into the
application bundle.

This directory is an engineering boundary, not the repository-wide license
inventory. Third-party material is recorded according to how Zeta consumes it:

| Kind | Canonical repository location | Release obligation |
| --- | --- | --- |
| Downloaded native runtime or vendored implementation | `third_party/<name>/` | Preserve its provenance, checksum, license, and required notices |
| Ordinary Rust or JavaScript dependency | Owning manifest and lockfile | Validate its license policy and include required notices in the product that distributes it |
| Component-specific bundled assets | The owning component, for example `zeta-rs/utils/typst/licenses/` | Copy the applicable license and notice texts into the release |
| Desktop release notices | `desktop/THIRD_PARTY_NOTICES.md` and `desktop/licenses/` | Ship them with the desktop application |

Do not add a package-manager dependency to this directory merely because it is
third-party. Add it here only when Zeta owns the download, verification,
vendoring, patching, or runtime-bundling path. A first-party wrapper remains
licensed under Zeta's root license; any license files stored beside that wrapper
must be clearly identified as applying to its upstream dependencies or bundled
assets.

| Runtime | Purpose | Distribution policy |
| --- | --- | --- |
| `bubblewrap` | Linux filesystem/network namespace enforcement | Required by canonical Linux releases |
| `pdfium` | Agent PDF extraction and page rendering | Required by PDF ingestion releases |
| `powershell` | Optional Windows PowerShell 7 runtime | Bundle only for releases that require a consistent `pwsh` runtime |
| `ripgrep` | Model-visible content and path search executable | Required in every canonical Zeta package |
| `wezterm` | Provenance for reused PTY implementation code | Do not bundle the WezTerm GUI application |

Downloaded artifacts are intentionally ignored under `third_party/.cache/`.
See each runtime's README before adding a new platform or updating a version.

When changing a bundled component, update its locked version and provenance,
review its transitive and asset licenses, and keep the component-local license
texts synchronized with every release-facing copy.
