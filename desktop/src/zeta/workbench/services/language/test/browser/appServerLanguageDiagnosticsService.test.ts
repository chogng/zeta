import assert from "node:assert/strict";
import test from "node:test";
import { URI } from "../../../../../base/common/uri.js";
import { TextPosition, TextRange } from "../../../../../editor/common/core/text.js";
import { LanguageDiagnosticSeverity } from "../../../../../editor/common/languages/languageResults.js";
import { TextModel } from "../../../../../editor/common/model/textModel.js";
import { type IServerEventApi } from "../../../../../platform/app-server/common/appServerApi.js";
import { type ILanguageApi } from "../../../../../platform/language/common/languageApi.js";
import { type IWorkspaceTrustService, type WorkspaceTrustSetting } from "../../../../../platform/workspaceTrust/common/workspaceTrustService.js";
import { WorkspaceContextService } from "../../../workspaces/browser/workspaceContextService.js";
import { type CodeIntelligenceDocumentSnapshot, type ICodeIntelligenceDocumentService } from "../../../codeIntelligence/common/codeIntelligenceDocumentService.js";
import { AppServerLanguageDiagnosticsService } from "../../browser/appServerLanguageDiagnosticsService.js";
import { type ServerNotification } from "../../../../../../../generated/app-server/types.js";

test("App Server diagnostics service synchronizes, filters revisions, and closes once", async () => {
  const events = new FakeServerEvents();
  const api = new FakeLanguageApi();
  const documents = new FakeCodeIntelligenceDocuments();
  using workspace = new WorkspaceContextService({ id: "workspace", uri: URI.file("C:\\project") });
  using service = new AppServerLanguageDiagnosticsService(api, events, workspace, documents);
  using model = new TextModel("fn main() {}\n");
  const resource = URI.file("C:\\project\\main.rs");
  using first = service.acquire(resource, "rust", model);
  using second = service.acquire(resource, "rust", model);
  await tick();
  assert.equal(api.synchronized.length, 1);
  assert.equal(api.diagnosticPulls.length, 1);
  assert.equal(api.synchronized[0]!.document.path, "main.rs");
  assert.deepEqual(documents.synchronized, api.synchronized.map(request => request.document));

  events.fire({ method: "language/diagnostics", params: { path: "main.rs", revision: 1, diagnostics: [{ range: { start: { lineIndex: 0, columnIndex: 3 }, end: { lineIndex: 0, columnIndex: 7 } }, severity: "error", message: "broken", code: "E1", source: "fixture" }] } });
  assert.equal(service.getDiagnostics(resource)?.diagnostics[0]!.message, "broken");
  using publisher = service.createPublisher(resource);
  publisher.update(1, [{ range: TextRange.from(TextPosition.at(0, 3), TextPosition.at(0, 7)), severity: LanguageDiagnosticSeverity.Error, message: "broken", code: "E1", source: "fixture" }]);
  assert.equal(service.getDiagnostics(resource)?.diagnostics.length, 1);
  assert.equal(service.getAllDiagnostics().length, 1);
  model.applyEdits([{ range: TextRange.emptyAt(TextPosition.at(0, 0)), text: "// " }]);
  assert.equal(service.getDiagnostics(resource)?.revision, 1);
  publisher.update(2, [{ range: TextRange.from(TextPosition.at(0, 0), TextPosition.at(0, 2)), severity: LanguageDiagnosticSeverity.Warning, message: "local warning", source: "syntax" }]);
  assert.equal(service.getDiagnostics(resource)?.revision, 2);
  assert.equal(service.getDiagnostics(resource)?.diagnostics[0]!.message, "local warning");

  events.fire({ method: "language/diagnostics", params: { path: "main.rs", revision: 0, diagnostics: [] } });
  assert.equal(service.getDiagnostics(resource)?.revision, 2);
  await new Promise(resolve => setTimeout(resolve, 200));
  assert.equal(api.synchronized.at(-1)!.document.revision, 2);
  second.dispose();
  await tick();
  assert.equal(api.closed.length, 0);
  first.dispose();
  await tick();
  assert.deepEqual(api.closed, [{ path: "main.rs" }]);
  assert.deepEqual(documents.closed, api.closed.map(request => request.path));
  assert.equal(service.getAllDiagnostics().length, 1);
  publisher.dispose();
  assert.equal(service.getAllDiagnostics().length, 0);
});

class FakeServerEvents implements IServerEventApi {
  private listener: ((event: ServerNotification) => void) | undefined;
  subscribe(listener: (event: ServerNotification) => void) { this.listener = listener; return { dispose: () => { this.listener = undefined; } }; }
  fire(event: ServerNotification): void { this.listener?.(event); }
}

class FakeCodeIntelligenceDocuments implements ICodeIntelligenceDocumentService {
  readonly synchronized: CodeIntelligenceDocumentSnapshot[] = [];
  readonly closed: string[] = [];
  async synchronize(document: CodeIntelligenceDocumentSnapshot) { this.synchronized.push(document); }
  async close(path: string) { this.closed.push(path); }
}

class FakeLanguageApi implements ILanguageApi {
  readonly synchronized: Parameters<ILanguageApi["synchronize"]>[0][] = [];
  readonly closed: Parameters<ILanguageApi["close"]>[0][] = [];
  readonly diagnosticPulls: Parameters<ILanguageApi["documentDiagnostics"]>[0][] = [];
  documentDiagnosticsError: Error | undefined;
  workspaceDiagnosticPulls = 0;
  workspaceDiagnosticsError: Error | undefined;
  workspaceReport: Awaited<ReturnType<ILanguageApi["workspaceDiagnostics"]>> = { supported: false, snapshots: [] };
  async synchronize(params: Parameters<ILanguageApi["synchronize"]>[0]): Promise<void> { this.synchronized.push(params); }
  async close(params: Parameters<ILanguageApi["close"]>[0]): Promise<void> { this.closed.push(params); }
  hover(): ReturnType<ILanguageApi["hover"]> { throw new Error("unused"); }
  completions(): ReturnType<ILanguageApi["completions"]> { throw new Error("unused"); }
  resolveCompletion(): ReturnType<ILanguageApi["resolveCompletion"]> { throw new Error("unused"); }
  executeCommand(): ReturnType<ILanguageApi["executeCommand"]> { throw new Error("unused"); }
  async documentDiagnostics(params: Parameters<ILanguageApi["documentDiagnostics"]>[0]): ReturnType<ILanguageApi["documentDiagnostics"]> { this.diagnosticPulls.push(params); if (this.documentDiagnosticsError) throw this.documentDiagnosticsError; return { revision: params.document.revision, kind: "unchanged", diagnostics: [] }; }
  async workspaceDiagnostics(): ReturnType<ILanguageApi["workspaceDiagnostics"]> { this.workspaceDiagnosticPulls += 1; if (this.workspaceDiagnosticsError) throw this.workspaceDiagnosticsError; return this.workspaceReport; }
  formatDocument(): ReturnType<ILanguageApi["formatDocument"]> { throw new Error("unused"); }
  formatRange(): ReturnType<ILanguageApi["formatRange"]> { throw new Error("unused"); }
  signatureHelp(): ReturnType<ILanguageApi["signatureHelp"]> { throw new Error("unused"); }
  inlayHints(): ReturnType<ILanguageApi["inlayHints"]> { throw new Error("unused"); }
  linkedEditingRanges(): ReturnType<ILanguageApi["linkedEditingRanges"]> { throw new Error("unused"); }
  semanticTokens(): ReturnType<ILanguageApi["semanticTokens"]> { throw new Error("unused"); }
  documentSymbols(): ReturnType<ILanguageApi["documentSymbols"]> { throw new Error("unused"); }
  codeLenses(): ReturnType<ILanguageApi["codeLenses"]> { throw new Error("unused"); }
  resolveCodeLens(): ReturnType<ILanguageApi["resolveCodeLens"]> { throw new Error("unused"); }
  documentLinks(): ReturnType<ILanguageApi["documentLinks"]> { throw new Error("unused"); }
  resolveDocumentLink(): ReturnType<ILanguageApi["resolveDocumentLink"]> { throw new Error("unused"); }
  documentColors(): ReturnType<ILanguageApi["documentColors"]> { throw new Error("unused"); }
  colorPresentations(): ReturnType<ILanguageApi["colorPresentations"]> { throw new Error("unused"); }
  foldingRanges(): ReturnType<ILanguageApi["foldingRanges"]> { throw new Error("unused"); }
  locations(): ReturnType<ILanguageApi["locations"]> { throw new Error("unused"); }
  hierarchy(): ReturnType<ILanguageApi["hierarchy"]> { throw new Error("unused"); }
  workspaceSymbols(): ReturnType<ILanguageApi["workspaceSymbols"]> { throw new Error("unused"); }
  prepareRename(): ReturnType<ILanguageApi["prepareRename"]> { throw new Error("unused"); }
  rename(): ReturnType<ILanguageApi["rename"]> { throw new Error("unused"); }
  codeActions(): ReturnType<ILanguageApi["codeActions"]> { throw new Error("unused"); }
  resolveCodeAction(): ReturnType<ILanguageApi["resolveCodeAction"]> { throw new Error("unused"); }
}

async function tick(): Promise<void> { await new Promise(resolve => setTimeout(resolve, 0)); }

test("App Server diagnostics service includes unopened workspace reports", async () => {
  const events = new FakeServerEvents();
  const api = new FakeLanguageApi();
  api.workspaceReport = { supported: true, snapshots: [{ path: "src/unopened.rs", diagnostics: [{ range: { start: { lineIndex: 2, columnIndex: 1 }, end: { lineIndex: 2, columnIndex: 4 } }, severity: "warning", message: "workspace warning", code: null, source: "fixture" }] }] };
  using workspace = new WorkspaceContextService({ id: "workspace", uri: URI.file("C:\\project") });
  using service = new AppServerLanguageDiagnosticsService(api, events, workspace);
  await tick();
  await tick();
  const resource = URI.file("C:\\project\\src\\unopened.rs");
  assert.equal(service.getDiagnostics(resource)?.revision, 0);
  assert.equal(service.getDiagnostics(resource)?.diagnostics[0]?.message, "workspace warning");
  assert.equal(service.getAllDiagnostics().length, 1);

  using model = new TextModel("fn open() {}\n");
  using acquisition = service.acquire(resource, "rust", model);
  await tick();
  assert.equal(service.getDiagnostics(resource), undefined);
});

test("App Server diagnostics service treats typed unavailable pulls as unsupported", async () => {
  const events = new FakeServerEvents();
  const api = new FakeLanguageApi();
  api.documentDiagnosticsError = new Error("LanguageRequestFailed");
  api.workspaceDiagnosticsError = new Error("LanguageServiceUnavailable");
  const reported: unknown[][] = [];
  const originalConsoleError = console.error;
  console.error = (...arguments_: unknown[]) => reported.push(arguments_);
  try {
    using workspace = new WorkspaceContextService({ id: "workspace", uri: URI.file("C:\\project") });
    using service = new AppServerLanguageDiagnosticsService(api, events, workspace);
    using model = new TextModel("fn main() {}\n");
    using acquisition = service.acquire(URI.file("C:\\project\\main.rs"), "rust", model);
    await tick();
    await tick();

    assert.equal(api.diagnosticPulls.length, 1);
    assert.ok(api.workspaceDiagnosticPulls > 0);
    assert.deepEqual(reported, []);
  } finally {
    console.error = originalConsoleError;
  }
});

test("App Server diagnostics service waits for a workspace folder before pulling workspace diagnostics", async () => {
  const events = new FakeServerEvents();
  const api = new FakeLanguageApi();
  using workspace = new WorkspaceContextService({ id: "empty-window" });
  using service = new AppServerLanguageDiagnosticsService(api, events, workspace);

  await tick();
  assert.equal(api.workspaceDiagnosticPulls, 0);

  workspace.updateWorkspace({ id: "workspace", uri: URI.file("C:\\project") });
  await tick();
  await tick();
  assert.ok(api.workspaceDiagnosticPulls > 0);
});

test("App Server diagnostics service gates Editor synchronization on Workspace Trust and replays open models after trust", async () => {
  const events = new FakeServerEvents();
  const api = new FakeLanguageApi();
  const trust = new FakeWorkspaceTrustService("workspace", "restricted");
  using workspace = new WorkspaceContextService({ id: "workspace", uri: URI.file("C:\\project") });
  using service = new AppServerLanguageDiagnosticsService(api, events, workspace, undefined, trust);
  using model = new TextModel("fn main() {}\n");
  using acquisition = service.acquire(URI.file("C:\\project\\main.rs"), "rust", model);

  await tick();
  await tick();
  assert.equal(api.synchronized.length, 0);
  assert.equal(api.workspaceDiagnosticPulls, 0);

  trust.setting = "trusted";
  events.fire({ method: "config/changed", params: { revision: 2, generation: 2 } });
  await tick();
  await tick();
  assert.equal(api.synchronized.length, 1);
  assert.equal(api.synchronized[0]!.document.path, "main.rs");
  assert.ok(api.workspaceDiagnosticPulls > 0);

  trust.setting = "restricted";
  events.fire({ method: "config/changed", params: { revision: 3, generation: 3 } });
  await tick();
  await tick();
  const synchronizedBeforeEdit = api.synchronized.length;
  model.applyEdits([{ range: TextRange.emptyAt(TextPosition.at(0, 0)), text: "// " }]);
  await new Promise(resolve => setTimeout(resolve, 200));
  assert.equal(api.synchronized.length, synchronizedBeforeEdit);
});

class FakeWorkspaceTrustService implements IWorkspaceTrustService {
  constructor(private readonly workspace: string, public setting: WorkspaceTrustSetting) {}
  async list() { return { revision: 1, entries: [{ workspace: this.workspace, root: "C:\\project", setting: this.setting }] }; }
  async set(): ReturnType<IWorkspaceTrustService["set"]> { throw new Error("unused"); }
  async forget(): ReturnType<IWorkspaceTrustService["forget"]> { throw new Error("unused"); }
}
