# Dedicated Sessions workbenches

`sessions/` is a top-level product layer beside `workbench/`, modelled after
VS Code's dedicated Agents Window. It may import reusable Workbench
contributions; `workbench/` must never import this directory. The regular
Workbench layout therefore remains untouched.

| Surface | Owner | Current composition |
| --- | --- | --- |
| Code Sessions | `browser/code/` | session list, agent transcript, and a focused development context |
| Academic Sessions | `browser/academic/` | research-session list, local literature import, PDF reader, native research browser, writing draft, and agent chat |
| Shared runtime | `browser/common/sessionsRuntime.ts` | one App Server-backed session/thread service and Chat service per Sessions page |

Each ordinary product Workbench entry imports only the small
`sessionTitlebarEntry` contribution. That action navigates to the matching
sibling Sessions HTML page. Electron Main includes both pages in its trusted
IPC entry allowlist, while `sessions/electron-browser/electronSessions.ts`
creates the dedicated host. Returning to Workbench performs the inverse page
navigation.

The Academic layout is deliberately fixed: library and research sessions on
the left; read, browse, and draft surfaces in the centre; writing agent on the
right. Its embedded browser is the existing main-owned `BrowserView` API, so
external pages stay isolated from renderer privileges. Imported PDF/BibTeX/RIS
files currently live only in the active renderer page; durable literature
library storage and citation parsing are future Academic-domain work, not
hidden Workbench state.

`browser/*WorkbenchSession.ts` remains the legacy initial profile consumed by
the normal Workbench. It is not the dedicated Sessions workbench and must not
gain Sessions-specific layout logic.
