# Code Sessions

`sessions/` is the Code product's top-level agent-workflow layer beside
`workbench/`, modelled after VS Code's dedicated Agents Window. It may import
reusable Workbench contributions; `workbench/` must never import this
directory. The regular Workbench layout therefore remains untouched.

| Surface | Owner | Current composition |
| --- | --- | --- |
| Code Sessions | `browser/code/` | session list, agent transcript, and a focused development context |
| Shared runtime | `browser/common/sessionsRuntime.ts` | one App Server-backed session/thread service and Chat service per Sessions page |

Only the Code Workbench entry imports `sessionTitlebarEntry`. In Electron it
opens the Code Sessions window; browser builds navigate to the sibling Code
Sessions page. `sessions/electron-browser/electronSessions.ts` creates that
dedicated host. Returning to Workbench closes the Sessions window.

Academic has no dedicated Sessions product surface; its initial layout is
owned next to its regular Workbench entry.
