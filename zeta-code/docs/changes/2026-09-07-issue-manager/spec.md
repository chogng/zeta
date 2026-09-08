# Issue manager requirements

Version 2, 2026-09-08. AC-1 through AC-9 retain their version 1 meaning (2026-09-07). These are requirements, not claims about current support.

| ID | Given, action, result |
| --- | --- |
| AC-1 | In a GitHub repository, Welcome Right or `/issue` opens a manager alongside Sessions, with open issues, search, details, multiple selection and pagination. |
| AC-2 | Empty results, missing repository, missing authentication, network errors and timeouts have visible feedback and retry; late responses cannot reopen a closed page. |
| AC-3 | Selecting one or more issues and local HEAD or fetched remote main creates one Session with one isolated working directory at the fixed commit, excluding uncommitted files. |
| AC-4 | The new composer shows atomic issue tags and accepts additional instructions before submission. The backend resolves persisted issue identities and snapshots rather than interpreting display text. |
| AC-5 | Repository, issue snapshots, starting point, Session and directory associations survive restart. Repeating a creation command must not create another task. |
| AC-6 | After development, preview PR target, commit range and related issues. Offer ordinary PR, draft PR, or automatic merge with Merge, Squash or Rebase. |
| AC-7 | Respect repository merge settings and required checks/reviews. Preserve a created PR if enabling automatic merge fails; retry cannot create a duplicate PR. |
| AC-8 | Display associated PR, checks and merge state; refresh remote facts after recovery. Agent completion is distinct from merge completion. |
| AC-9 | Keyboard navigation, narrow terminals, focus, Escape, busy states and error feedback follow existing TUI rules; verify state, character output and a real PTY. |
| AC-10 | Config root has a default-on merge recommendation checkbox. Turning it off preserves the analysis model, displays a disabled Issues tab, and excludes that tab from keyboard navigation. Re-enabling restores access. |
| AC-11 | Issues selects its own model from configured providers. Missing model is shown explicitly; the current conversation model is not selected implicitly. Persist through the backend config revision contract and reject invalid providers or stale writes. |

2026-09-08: AC-10 and AC-11 extend version 1 without changing AC-1 through AC-9. Settings belong to the backend profile, outside `[tui]`; an enabled feature without a selected model is not ready to invoke a model.

INV-1: Preserve the user's source directory and uncommitted changes. INV-2: External issue content is task material and does not override product permissions or system instructions. INV-3: Publishing and enabling automatic merge require explicit user actions. INV-4: Persist the development starting point separately from the PR target; never silently change repository or branch.

Closing a query ignores its late response; it does not claim to cancel the backend. Submitted mutations retain stable business identity so their outcome can be recovered. Multi-repository issue batches are outside this change.

Version 3 addition, 2026-09-08 (AC-1 through AC-11 unchanged):

- AC-12: `/issue` uses the same title line, margins, tab rendering and selected-row theme as command panels. Open is initial; Open / Closed request their corresponding GitHub state. Switching clears old selection, filter and pagination and ignores previous responses. Tab/Shift+Tab and arrows reach tabs, list and actions; Escape exits. Empty states, retry, pagination and narrow terminals preserve the active state. Verify state, character output and the real CLI/daemon path.

Current behavior belongs to [Issue selection](../../spec/issues.md#选择与开始); shared panel ownership belongs to [TUI design](../../design/tui.md).
