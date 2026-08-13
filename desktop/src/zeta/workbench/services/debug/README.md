# Workbench Debug service

The system-level ownership and current user-visible status are documented in [`docs/debugging.md`](../../../../../../docs/debugging.md). This README owns the Renderer implementation contract.

`common/debugService.ts` defines the small frontend contract `IDebugService`. `browser/debugService.ts` owns workspace launch configurations, breakpoints, the active session, and updates confirmed by the adapter. `browser/debugAdapterSession.ts` owns DAP request pairing, initialization, breakpoints, execution control, stack/scopes/variables, output, termination, and reverse-request dispatch. `browser/debugTerminalLauncher.ts` validates `runInTerminal` and delegates process presentation to `ITerminalService`.

The Code product installs these services through `registerWorkbenchServiceContribution` and separately contributes the browser, Electron renderer, and Electron main transport adapters. The shared Workbench, editor, Renderer hosts, and Electron main application do not install Debug semantics or transport by default. `DebugBreakpointDecorationProvider` adapts `IDebugService` to the editor's generic composable gutter contract.

The implementation intentionally requires an explicit `debugAdapter.program` and optional `debugAdapter.args` in `.vscode/launch.json`. Every remaining configuration property is forwarded to the adapter after `${workspaceFolder}` and `${workspaceFolderBasename}` expansion. Adapter program/arguments receive the same expansion.

Run the Debug service tests with the desktop unit runner or target the compiled files under `services/debug/test`. The tests use a fake DAP process boundary and assert initialization, breakpoint clearing, omitted stopped-thread recovery, and stack parsing.

Current limitations include one active session, in-memory breakpoints, one displayed variable level at a time, no watch/evaluate console, no exception breakpoints, no compounds, no source-reference retrieval, and no extension-driven adapter discovery.
