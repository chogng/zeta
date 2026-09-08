# Issue development and PRs

Status: implementation in progress. Source: [issue #5](https://github.com/chogng/zeta/issues/5) and the user's decisions in this task on 2026-09-07.

The CLI/TUI lets the user select multiple issues, develop them together in one Session, and choose how to create a PR. The backend owns durable associations and execution; the TUI owns interaction. Other product interfaces and an independent PR review browser are outside this change.

The user confirmed that the starting point is either local HEAD or freshly fetched remote main. A new Session presents atomic `[issue #3]` composer tags and waits for the user to add instructions and send.

2026-09-08 scope addition: recommend related issues using an explicitly selected analysis model. The user chose a dedicated Issues configuration tab, with the enable checkbox on the Config root page, enabled by default. Turning it off disables the Issues tab and retains the model selection; ordinary issue selection remains available. The analysis implementation and the configuration UI have separate acceptance evidence.

See [requirements](spec.md), [implementation](plan.md), and [evidence](verification.md). On completion, merge current requirements into `zeta-code/docs/spec/issues.md` and update `design/tui.md`. The shared API remains documented in `docs/zeta-app-server-api.md`.

2026-09-08 panel correction: the user requested an actual Issue panel matching existing UI and Open / Closed tabs. Reuse panel chrome and TabList, query the selected remote state, and keep list selection and stale-response ownership in the Issue manager. This extends the unfinished manager work; existing configuration and PR evidence remains historical.
