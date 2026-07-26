# Third-party runtime artifacts

This directory stores the **source-controlled metadata** for native runtimes. It
does not store downloaded archives or platform binaries. CI and release builds
must fetch each locked artifact, verify its SHA-256 digest, and copy it into the
application bundle.

| Runtime | Purpose | Distribution policy |
| --- | --- | --- |
| `pdfium` | Agent PDF extraction and page rendering | Required by PDF ingestion releases |
| `powershell` | Optional Windows PowerShell 7 runtime | Bundle only for releases that require a consistent `pwsh` runtime |
| `wezterm` | Provenance for reused PTY implementation code | Do not bundle the WezTerm GUI application |

Downloaded artifacts are intentionally ignored under `third_party/.cache/`.
See each runtime's README before adding a new platform or updating a version.
