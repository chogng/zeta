# Implementation plan

Status: in progress, against [specification](spec.md) version 1.

- [x] GitHub backend: repository discovery, issue/comment pagination, PR creation and status, bounded subprocess output and timeouts. AC-1, AC-2, AC-6 through AC-8.
- [x] Durable task associations and command identity; create the Session through the existing Thread directory binding boundary. AC-3, AC-5.
- [x] Typed protocol registry and Rust client, existing serialization/error conventions, generated protocol artifacts.
- [x] TUI manager, search, selection, details and starting-point actions; reject stale requests. AC-1, AC-2, AC-9.
- [x] Atomic composer issue tags and structured submission; restore unsent issue context. AC-4, AC-5.
- [x] PR preview, explicit creation choice, automatic merge and remote status refresh. AC-6 through AC-8.
- [ ] Targeted tests, package builds, real PTY and current documentation. Record actual evidence per requirement.

2026-09-08 configuration addition:

- [x] Persist a default-on recommendation switch and independent model in backend Issue settings.
- [x] Root Config checkbox and disabled Issues tab, retaining the selected model.
- [x] Configured-provider model picker with revision checks and stale-result handling.
- [x] Finish the configuration PTY acceptance and record the final candidate.
- [ ] Implement similarity analysis and recommendation execution.

The TUI owns presentation. GitHub owns external-service access. Existing Git/worktree modules own code and directories. App-server coordinates these with Session creation. Reuse ThreadWorktreeBinder rather than creating a second binding system.

Code investigation: ordinary root Thread source_for uses DirSnapshot; issue tasks require an immutable commit tree. SessionCreateParams currently contains only command_id/title. A dedicated issue task entry must preserve ordinary Session semantics.

2026-09-08 panel correction (AC-12, specification version 3):

- [x] Extract existing command-panel chrome for both callers; reuse TabList and themed selection.
- [x] Carry explicit Open / Closed state through TUI, protocol, app-server and GitHub.
- [x] Cover focus, stale responses, empty/retry/pagination and narrow render output.
- [x] Run targeted builds/tests, protocol generation and the actual CLI selection scenario; append evidence.
