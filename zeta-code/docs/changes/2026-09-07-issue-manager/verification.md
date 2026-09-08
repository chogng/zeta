# Verification

Requirements: [specification](spec.md) version 1, 2026-09-07. Status: incomplete.

Baseline: `7f935080d0c39b1cdfebac25d93dceb7235bafae`. Changes are uncommitted in the requested checkout. No real GitHub PR was published.

## 2026-09-08 implementation checks

The initial construction checks `just check zeta-github`, `just check zeta-git`, `just check zeta-app-server`, and `just check zeta-tui` completed successfully. Later changes require the final candidate checks below; these early results are not final acceptance.

| Command | Observed result |
| --- | --- |
| `just test zeta-github` (initial IO implementation) | 5 passed: repository validation, invalid requests without subprocesses, draft merge rejection, issue/PR separation and comment pagination, structured remote errors |
| `just test zeta-git --lib branch::tests` | 2 passed: fixed local HEAD with original dirty files preserved; fetched main without changing local HEAD |
| `just test zeta-state --lib issue_tasks` | 1 passed: combined selection survives reopening the database, identical commands replay, changed preparation is rejected |
| `just test zeta-tui --lib` (early candidate) | 702 passed, 6 failed, 1 ignored. Three failures involved placing new commands first; two directory tests failed during Session creation; one existing terminal protocol test lacked a controlling terminal |
| `just test zeta-tui --lib add_dir_adds_lists_and_removes_the_exact_session_directory -- --nocapture` | Passed in isolation without a directory behavior change |
| `just test zeta-tui --lib panel_add_uses_server_paths_preserves_permissions_and_reports_missing_directories` | Passed in isolation without a directory behavior change |
| `just test zeta-tui --lib bare_slash_renders_the_first_command_window` | Passed after preserving existing command order |
| `just test zeta-tui --lib slash_popup_clears_covered_transcript_rows_edge_to_edge` | Passed against the unchanged committed snapshot; no pending snapshot remains |
| `just test zeta-tui --lib builtins_follow_enum_presentation_order` | Passed with the two new commands appended |
| `just test zeta-tui --lib issue` | 8 passed: multi-selection, retries, closed/replaced query handling, atomic tags, deletion, queue restoration and plain-text spoof prevention |
| `just test zeta-app-server --lib prepared_issue_task_recovers_exact_code_and_scoped_context_without_duplicate_sessions` | Passed for both repository root and subdirectory: exact committed content, unchanged source files, one Session, stored issue context, rejection of unassociated numbers, and recovery of the same directory |
| `just generate-protocol` | Completed; JSON schema, TypeScript contract and decoder regenerated |
| Local Markdown target check | No missing file targets in changed Markdown |

The early parallel directory failures were not reproduced by the two isolated reruns. Their cause is not established. An intermediate formatting pass removed protocol imports; generation/build rejected that candidate, the imports were restored, and protocol generation passed again. No failing candidate is recorded as accepted.

## Candidate under final verification

Code SHA-256 recorded before CLI scenario execution:
`51844dbb1590dbb296308247f9ea5c900aa81cdd037d393903f62cbf157d8010`.
The fingerprint hashes sorted changed/untracked file names and contents, separated by NUL, excluding Markdown and pending `.snap.new` files. Documentation does not hash itself.

CLI compilation, actual PTY scenarios, final protocol/GitHub tests, and the terminal-dependent rerun are still pending. AC-1 through AC-9 have implementation, but final acceptance is incomplete until these results are recorded. GitHub interactions in PTY tests use a scoped offline provider and a local bare Git remote; they do not prove live GitHub permissions or branch protection settings.

## 2026-09-08 Config root switch and Issues tab (AC-10, AC-11)

The checkout was updated by another task to `b00ee076e5f594cc071b2800dd01b7d2dd135a96`. The earlier candidate remains historical evidence; the following results cover the configuration addition on the updated checkout.

| Verification | Result |
| --- | --- |
| `just check zeta-tui` | Passed; includes the backend configuration and protocol consumers |
| `just test zeta-config --lib issue_config_defaults_on_and_preserves_its_model_across_disable_and_restart` | Passed: default on, no implicit model, missing provider rejected, disable/re-enable retains the model, stale write rejected, profile restart restores settings, conversation model unchanged |
| `just test zeta-tui --lib issue_config` | 6 passed: root checkbox, disabled tab navigation and programmatic focus, model retention, stale picker results, configured-provider model selection, backend request shape and no model-catalog call while disabled |
| `just test zeta-tui --lib config::editor::tests` | 12 passed, including existing provider, language and keyboard interaction regressions |
| `just test-tui actual_tui_issue_config_switch_gates_its_tab -- --nocapture` | Passed in a real macOS PTY: default-enabled Issues tab, off/on persisted in TOML, disabled tab skipped, model configuration visible after re-enable and resizing; no model request occurred |
| `just generate-protocol` | Passed after registering Issue configuration and workflow declarations in the TypeScript binding list |
| `just test zeta-app-server-protocol --lib issue_config_and_workflow_method_types_are_declared_in_typescript` | Passed |
| `corepack pnpm --dir zeta-ts exec tsc --noEmit --skipLibCheck --target ES2022 --module NodeNext --moduleResolution NodeNext generated/app-server/index.ts` | Passed |

An initial build failed because the disk was full; space subsequently became available and builds resumed. An initial configuration test exposed validation against an empty provider registry. Configuration now follows the existing contract: the reference must identify a configured provider, while model availability belongs to the model catalog/runtime; the same test then passed. The generator initially rejected the missing Issue declaration, and the successful regeneration and TypeScript check above validate the corrected artifacts.

AC-10 and AC-11 are accepted for the tested configuration scope. Similarity-analysis execution is not implemented yet. The original PR end-to-end acceptance is still separate: rerunning it on the new terminal layout exposed missing Issue-page height allocation, which is being corrected and verified; this does not invalidate the successful Config scenario.

### Follow-up validation on the updated terminal layout

- `just test zeta-tui --lib issue_manager_reserves_page_height_when_the_transcript_is_empty`: passed at 100×32 and 60×16. Issue pages now use the same full-page height rule as the Session manager instead of the short, empty-transcript height.
- `just test-tui actual_tui_issue_auto_squash_retry_preserves_the_created_pr -- --nocapture`: passed. The real CLI reads two issues through the offline GitHub adapter, creates a task from fetched main, submits the issue context, approves a file tool, commits task changes, pushes to the local bare remote, and creates one PR. Two automatic-Squash attempts fail explicitly while the PR creation count stays one and source files remain untouched.
- The final `just test zeta-tui --lib issue_config` rerun passed all 6 tests, including the model-picker response guard after leaving the Issues tab.

Current source fingerprint before final PR-mode checks:
`6b8d24f0fe8ebba0d46583500ab2c3992034aa0584a24e3223794a415c52041e`.
This fingerprint includes staged and unstaged changes against HEAD and untracked source files, excluding Markdown and `.snap.new` files.

### Final PR-mode and selection results

Requirements: [specification](spec.md) version 2, 2026-09-08. The source candidate is the fingerprint above; these results extend the earlier evidence without replacing it.

- `just test-tui actual_tui_issue_regular_and_draft_prs -- --nocapture`: passed. Both ordinary and draft creation completed through the real CLI and local daemon; the offline provider recorded the correct draft flag, both issue references, and no automatic-merge request.
- `just test-tui actual_tui_issue_selection_creates_one_draft_without_touching_source_changes -- --nocapture`: passed on the updated layout. Multiple selected issues produce one Session with editable composer context; original uncommitted files remain unchanged and starting the Session does not invoke the conversation model.
- Both commands built the matching daemon and CLI successfully. `git diff --check` passed, and no unmerged paths remained.

The Config root checkbox and Issues model settings are complete for the tested scope. The updated terminal layout, selection, ordinary PR, draft PR, and failed automatic-Squash retry scenarios passed on macOS. GitHub behavior was exercised through the offline provider and local bare remote, not a live repository. Similarity-analysis execution remains unimplemented; these configuration results do not imply that recommendations are being generated. Review performed in this task was self-review.

## 2026-09-08 Issue panel and Open / Closed tabs (AC-12)

Requirements: specification version 3. Baseline remains `b00ee076e5f594cc071b2800dd01b7d2dd135a96`; source fingerprint `69ae6eb918bf14837e04b27c56e3b91587eb1dd1dbb063ba13c7ae4a35dc210e` hashes the 85 changed/untracked source files using the method above, excluding Markdown and pending snapshots. Existing work in this checkout is retained.

The Issue manager and command panels now share `widgets/panel.rs` for the title line and layout; those helpers moved out of `app/command_panel.rs`. TabList owns tab interaction and rendering, SearchBox owns filter editing, and the existing list style owns row emphasis. Issue state is explicit through the request, protocol, server and GitHub query. Switching resets selection, filter and pagination and rejects old responses.

| Verification | Result |
| --- | --- |
| `just check zeta-tui` | Passed for the panel/state integration; the subsequent CLI build validates the final source |
| `just test zeta-tui --lib issue` | 21 passed, including 5 new tests for state switching, stale replies, empty/retry/pagination, focus, search, and character/geometry/style assertions at 100×24, 60×12, 20×12 and 1×1 |
| `just test zeta-github --lib` | 6 passed; the issue-reader test now verifies Open page 1 and Closed page 2 against state-sensitive responses, PR exclusion and comment pagination |
| `just test zeta-app-server-protocol --lib issue` | 2 passed; required state accepts only Open/Closed and the TypeScript declarations include the enum |
| `just test zeta-tui --lib command_panel` | 9 passed; existing shared-title, wrapped-tab, keyboard, theme, layout and terminal-history checks |
| `just generate-protocol` | Passed; both protocol fixtures and frontend bindings regenerated |
| `corepack pnpm --dir zeta-ts exec tsc --noEmit --skipLibCheck --target ES2022 --module NodeNext --moduleResolution NodeNext generated/app-server/index.ts` | Passed |
| `just test-tui actual_tui_issue_selection_creates_one_draft_without_touching_source_changes -- --nocapture` | Passed in a real macOS PTY at 100×32 and 60×16: shared title, Open → Closed → Open, distinct remote results, multi-selection, one new Session, unchanged source file and no model call; matching daemon and CLI builds passed |
| Documentation and whitespace | Touched documents have no missing local file links; `git diff --check` passed |

The terminal scenario uses an offline GitHub provider and preserves the existing source-change and no-model-call assertions. No live GitHub PR is published. Review is self-review. Disk pressure during validation was handled by removing completed TUI incremental cache and older TUI object files under the ignored build directory; source files and current test executables were retained.

Final AC-12 result: accepted for the tested macOS TUI scope. `just test-tui actual_tui_issue_regular_and_draft_prs -- --nocapture` also passed both ordinary and draft PR flows after the shared panel change. Both matching builds passed again. The final source fingerprint is unchanged; all 9 touched documents have valid local file links and `git diff --check` passes. The new panel work is complete; the separately recorded similarity-analysis implementation remains outside this correction.


## 2026-09-08 PR #16：Issue 上下文失败重试

来源：[评审意见](https://github.com/chogng/zeta/pull/16#discussion_r3957752033)。上一轮允许失败请求再次加载，但每次终端轮询都会重新发起，造成请求和错误正文持续增长。本轮只修复这一恢复路径，延续本次工作记录。

基线：`637d4164d5925a1c4cd5b821ca3457d39f433961`；代码范围为 `zeta-code/tui/src/app/driver.rs` 和 `zeta-code/tui/src/app/driver/tests.rs`。现行行为见 [Issue 选择与开始](../../spec/issues.md#选择与开始)，职责仍由 [TUI 后台请求](../../design/tui.md#后台请求与旧结果) 中的 AppDriver 承担。

| 验收项 | 必要行为 | 验证 |
| --- | --- | --- |
| R-1 | 连续失败按 1、2、4、8、16、30 秒等待，之后保持 30 秒；从失败完成时计时 | 通过：`just test zeta-tui --lib issue_context` |
| R-2 | 两分钟的 25 毫秒轮询只触发 8 次立即失败的请求，正文仅增加一条错误 | 通过：`just test zeta-tui --lib issue_context` |
| R-3 | 延迟重试成功后停止请求；空结果也停止请求 | 通过：`just test zeta-tui --lib issue_context` |
| R-4 | 切换 Thread 重置等待，旧成功或失败不能修改当前状态，切回也重新加载 | 通过：`just test zeta-tui --lib issue_context` |

候选代码指纹：`9b0d4958e0a5a51cf92b3c26f34f1c6e28587198b564ee99e3d092cbefc76c1d`，为上述两个 Rust 文件相对基线的 `git diff --binary` 输出的 SHA-256，不包含文档。`just test zeta-tui --lib issue_context` 完成，5 项通过，包含原有的发送后拒绝迟到标签回归。`just check zeta-tui` 完成通过；定向 `rustfmt --check`、`git diff --check` 与 3 份修改文档的 49 个本地文件链接检查通过。本轮 R-1 至 R-4 已完成验收。测试使用可控的单调时间，并将失败事件送入实际 App 正文状态；本轮没有新增界面或终端输出布局，不以截图验收，也没有重跑真实 PTY 或访问真实 GitHub Issue 的场景。审查方式为自查。
