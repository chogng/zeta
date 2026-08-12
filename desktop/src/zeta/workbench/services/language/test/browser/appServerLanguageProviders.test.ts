import assert from "node:assert/strict";
import test from "node:test";
import { URI } from "../../../../../base/common/uri.js";
import { TextPosition, TextRange } from "../../../../../editor/common/core/text.js";
import { LanguageDiagnosticSeverity } from "../../../../../editor/common/languages/languageResults.js";
import { TextModel } from "../../../../../editor/common/model/textModel.js";
import { LanguageFeaturesService } from "../../../../../editor/common/services/languageService.js";
import { type ILanguageApi } from "../../../../../platform/language/common/languageApi.js";
import { WorkspaceContextService } from "../../../workspaces/browser/workspaceContextService.js";
import { AppServerLanguageProviders } from "../../browser/appServerLanguageProviders.js";

const DTO_RANGE = Object.freeze({ start: Object.freeze({ lineIndex: 0, columnIndex: 0 }), end: Object.freeze({ lineIndex: 0, columnIndex: 5 }) });

test("App Server language providers map cross-resource locations without double-encoding paths", async () => {
  using languages = new LanguageFeaturesService();
  using workspace = new WorkspaceContextService({ id: "workspace", uri: URI.file("C:\\project") });
  const api = new FakeLanguageApi();
  using providers = new AppServerLanguageProviders(languages, api, workspace);
  using model = new TextModel("value");
  using navigation = languages.createLanguageNavigationService(model, URI.file("C:\\project\\main file.ts"));

  const locations = await navigation.provideDefinition("typescript", TextPosition.at(0, 2));

  assert.equal(api.locationRequests.length, 1);
  assert.equal(api.locationRequests[0]!.document.path, "main file.ts");
  assert.equal(api.locationRequests[0]!.document.text, "value");
  assert.equal(locations[0]!.resource.toString(), "file:///C:/project/src/with%20space.ts");
  assert.equal(locations[0]!.selectionRange?.end.columnIndex, 5);
});

test("App Server workspace symbols query every supported Code language and deduplicate results", async () => {
  using languages = new LanguageFeaturesService();
  using workspace = new WorkspaceContextService({ id: "workspace", uri: URI.file("C:\\project") });
  const api = new FakeLanguageApi();
  using providers = new AppServerLanguageProviders(languages, api, workspace);
  using symbols = languages.createWorkspaceSymbolService();

  const result = await symbols.provideWorkspaceSymbols("answer");

  assert.deepEqual(api.workspaceSymbolLanguages.sort(), ["javascript", "javascriptreact", "json", "jsonc", "rust", "shell", "typescript", "typescriptreact"]);
  assert.equal(result.length, 1);
  assert.equal(result[0]!.resource.toString(), "file:///C:/project/src/with%20space.ts");
});

test("App Server rename and code actions preserve ordered workspace file operations", async () => {
  using languages = new LanguageFeaturesService();
  using workspace = new WorkspaceContextService({ id: "workspace", uri: URI.file("C:\\project") });
  const api = new FakeLanguageApi();
  using providers = new AppServerLanguageProviders(languages, api, workspace);
  using model = new TextModel("value");
  const resource = URI.file("C:\\project\\main.ts");
  using rename = languages.createRenameService(model, resource);
  using actions = languages.createCodeActionService(model, resource);

  const preparation = await rename.prepareRename("typescript", TextPosition.at(0, 2));
  const edit = await rename.provideRenameEdits("typescript", TextPosition.at(0, 2), "renamed");
  const available = await actions.provideCodeActions("typescript", TextRange.from(TextPosition.at(0, 0), TextPosition.at(0, 5)), [{ range: TextRange.from(TextPosition.at(0, 0), TextPosition.at(0, 5)), severity: LanguageDiagnosticSeverity.Warning, message: "demo", source: "test" }]);

  assert.equal(preparation?.placeholder, "value");
  assert.equal(edit.entries[0]!.kind, "create");
  assert.equal(edit.entries[1]!.kind, "textDocument");
  assert.equal(available[0]!.edit?.entries[0]!.kind, "rename");
  assert.equal(api.codeActionRequests[0]!.diagnostics[0]!.severity, "warning");
});

test("App Server language providers do not send documents above their transport limit", async () => {
  using languages = new LanguageFeaturesService();
  using workspace = new WorkspaceContextService({ id: "workspace", uri: URI.file("C:\\project") });
  const api = new FakeLanguageApi();
  using providers = new AppServerLanguageProviders(languages, api, workspace);
  using model = new TextModel("界".repeat(Math.ceil((10 * 1024 * 1024 + 1) / 3)));
  using navigation = languages.createLanguageNavigationService(model, URI.file("C:\\project\\large.ts"));

  assert.deepEqual(await navigation.provideDefinition("typescript", TextPosition.at(0, 0)), []);
  assert.equal(api.locationRequests.length, 0);
});

class FakeLanguageApi implements ILanguageApi {
  readonly locationRequests: Parameters<ILanguageApi["locations"]>[0][] = [];
  readonly codeActionRequests: Parameters<ILanguageApi["codeActions"]>[0][] = [];
  readonly workspaceSymbolLanguages: string[] = [];

  async locations(params: Parameters<ILanguageApi["locations"]>[0]): ReturnType<ILanguageApi["locations"]> {
    this.locationRequests.push(params);
    return { revision: params.document.revision, locations: [{ path: "src/with space.ts", range: DTO_RANGE, selectionRange: DTO_RANGE }] };
  }

  async hierarchy(params: Parameters<ILanguageApi["hierarchy"]>[0]): ReturnType<ILanguageApi["hierarchy"]> {
    return { revision: params.document.revision, entries: [] };
  }

  async workspaceSymbols(params: Parameters<ILanguageApi["workspaceSymbols"]>[0]): ReturnType<ILanguageApi["workspaceSymbols"]> {
    this.workspaceSymbolLanguages.push(params.languageId);
    return { symbols: [{ name: "answer", symbolKind: 12, containerName: "demo", path: "src/with space.ts", range: DTO_RANGE }] };
  }

  async prepareRename(): ReturnType<ILanguageApi["prepareRename"]> {
    return { preparation: { range: DTO_RANGE, placeholder: "value" } };
  }

  async rename(): ReturnType<ILanguageApi["rename"]> {
    return { entries: [
      { kind: "create", path: "created.ts", existing: "error" },
      { kind: "textDocument", document: { path: "main.ts", expectedText: "value", edits: [{ range: DTO_RANGE, newText: "renamed" }] } },
    ] };
  }

  async codeActions(params: Parameters<ILanguageApi["codeActions"]>[0]): ReturnType<ILanguageApi["codeActions"]> {
    this.codeActionRequests.push(params);
    return { actions: [{ title: "Move file", kind: "refactor.move", isPreferred: true, disabledReason: null, edit: { entries: [{ kind: "rename", source: "main.ts", target: "moved.ts", existing: "error" }] }, providerData: { id: 1 } }] };
  }

  async resolveCodeAction(): ReturnType<ILanguageApi["resolveCodeAction"]> {
    return { title: "Resolved", kind: null, isPreferred: false, disabledReason: null, edit: null, providerData: null };
  }
}
