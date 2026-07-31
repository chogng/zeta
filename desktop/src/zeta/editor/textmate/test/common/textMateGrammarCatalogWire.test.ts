import { strict as assert } from "node:assert";
import { readFile } from "node:fs/promises";
import test from "node:test";
import * as onigurumaNamespace from "vscode-oniguruma";
import { type IOnigLib } from "vscode-textmate";
import { Emitter, type Event } from "../../../../base/common/event.js";
import { DisposableOwner, DisposableStore } from "../../../../base/common/lifecycle.js";
import { LanguageAnalysisProviderModuleHost, LanguageAnalysisProviderModuleRegistry } from "../../../alpha/common/languageAnalysisProviderModules.js";
import { LanguageAnalysisProviderModuleWireServer } from "../../../alpha/common/languageAnalysisProviderModuleWire.js";
import { LanguageAnalysisProviderRegistry, type LanguageAnalysisProviderRequest } from "../../../alpha/common/languageAnalysisProviders.js";
import { LanguageAnalysisProviderWorker, LanguageAnalysisService } from "../../../alpha/common/languageAnalysisService.js";
import { languageAnalysisWireCodec } from "../../../alpha/common/languageAnalysisWire.js";
import { LanguageRequestStatus } from "../../../alpha/common/languageRequestCoordinator.js";
import { LanguageWorkerWireServer, type LanguageWorkerWireClientPort } from "../../../alpha/common/languageWorkerWire.js";
import { TextPosition, TextRange } from "../../../alpha/common/text.js";
import { TextModel } from "../../../alpha/common/textModel.js";
import { createTextMateAnalysisModule } from "../../common/textMateAnalysisModule.js";
import { TextMateAnalysisModuleWorkerClient } from "../../common/textMateAnalysisModuleWorkerClient.js";
import { materializeTextMateGrammarCatalog, TextMateGrammarCatalogModel, type TextMateGrammarCatalog } from "../../common/textMateGrammarCatalog.js";
import { TextMateGrammarCatalogStore } from "../../common/textMateGrammarCatalogStore.js";
import { TextMateGrammarCatalogWireClient, TextMateGrammarCatalogWireServer } from "../../common/textMateGrammarCatalogWire.js";
import { TextMateGrammarRegistry } from "../../common/textMateGrammarRegistry.js";
import { TextMateTokenizationService } from "../../common/textMateTokenizationService.js";

const onigurumaRuntime = (onigurumaNamespace as unknown as { readonly default?: typeof onigurumaNamespace }).default ?? onigurumaNamespace;
const { createOnigScanner, createOnigString, loadWASM } = onigurumaRuntime;
const onigLib = initializeOnigLib();

test("Grammar catalog model and store replace immutable revisions atomically", () => {
  using model = new TextMateGrammarCatalogModel();
  using store = new TextMateGrammarCatalogStore();
  const revisions: number[] = [];
  using listener = model.onDidChangeCatalog(catalog => revisions.push(catalog.revision));
  const first = grammarCatalog(1, "keyword.control.demo");

  model.replace(first);
  store.replace(model.currentCatalog);
  const firstSnapshot = store.currentSnapshot;

  assert.equal(Object.isFrozen(model.currentCatalog.grammars), true);
  assert.equal(firstSnapshot.getDefinitionForLanguage("demo")?.scopeName, "source.demo");
  assert.equal(store.catalogRevision, 1);
  assert.throws(() => model.replace(first), /revision must increase/);
  assert.throws(() => store.replace(first), /revision must increase/);
  assert.throws(() => model.replace({
    revision: 2,
    grammars: [
      ...first.grammars,
      { ...first.grammars[0]!, content: demoGrammar("string.quoted.demo") },
    ],
  }), /Duplicate/);
  assert.equal(model.currentCatalog.revision, 1);
  assert.equal(store.currentSnapshot, firstSnapshot);

  model.replace(grammarCatalog(2, "string.quoted.demo"));
  store.replace(model.currentCatalog);
  assert.notEqual(store.currentSnapshot, firstSnapshot);
  assert.deepEqual(revisions, [1, 2]);
});

test("Grammar registry snapshots materialize transferable catalog content", async () => {
  using registry = new TextMateGrammarRegistry();
  using root = registry.register({
    scopeName: "source.demo",
    languageId: "demo",
    loadGrammar: () => demoGrammar("keyword.control.demo"),
  });
  using injection = registry.register({
    scopeName: "source.demo.todo",
    injectTo: ["source.demo"],
    loadGrammar: () => ({
      scopeName: "source.demo.todo",
      injectionSelector: "L:comment",
      patterns: [],
      repository: {
        $self: { patterns: [] },
        $base: { patterns: [] },
      },
    }),
  });

  const catalog = await materializeTextMateGrammarCatalog(registry.currentSnapshot, 3, new AbortController().signal);

  assert.equal(catalog.revision, 3);
  assert.deepEqual(catalog.grammars.map(grammar => [grammar.scopeName, grammar.languageId, grammar.injectTo]), [
    ["source.demo", "demo", []],
    ["source.demo.todo", undefined, ["source.demo"]],
  ]);
  assert.equal(JSON.parse(catalog.grammars[1]!.content).scopeName, "source.demo.todo");
});

test("Grammar catalog wire clones catalogs and poisons stale clients", async () => {
  const [clientPort, serverPort] = createPortPair();
  using store = new TextMateGrammarCatalogStore();
  using server = new TextMateGrammarCatalogWireServer(serverPort, store);
  const invalidations: Error[] = [];
  using client = new TextMateGrammarCatalogWireClient(clientPort, error => invalidations.push(error));
  const catalog = grammarCatalog(1);

  await client.replaceCatalog(catalog);
  assert.equal(store.catalogRevision, 1);
  assert.notEqual((clientPort.sentMessages[0] as { catalog: unknown }).catalog, catalog);
  await assert.rejects(client.replaceCatalog(catalog), /revision must increase/);
  assert.equal(invalidations.length, 1);
  assert.throws(() => client.replaceCatalog(grammarCatalog(2)), /already disposed/);
});

test("Catalog-gated module Worker selects TextMate and falls back dynamically", async () => {
  using resources = new DisposableStore();
  const providers = resources.add(new LanguageAnalysisProviderRegistry());
  const modules = resources.add(new LanguageAnalysisProviderModuleRegistry());
  const grammarStore = resources.add(new TextMateGrammarCatalogStore());
  const tokenization = resources.add(new TextMateTokenizationService(grammarStore, onigLib));
  resources.add(modules.register(createTextMateAnalysisModule(tokenization)));
  resources.add(modules.register({
    id: "test.fallback",
    load: () => [{
      id: "test.fallback",
      languageIds: ["*"],
      provideTokens: (request: LanguageAnalysisProviderRequest) => ({
        tokens: [{
          range: TextRange.from(TextPosition.at(0, 0), TextPosition.at(0, request.snapshot.getText().length)),
          tokenType: "fallback",
          modifiers: [],
        }],
      }),
    }],
  }));
  const host = resources.add(new LanguageAnalysisProviderModuleHost(modules, providers));
  const [clientPort, serverPort] = createPortPair();
  resources.add(new LanguageWorkerWireServer(serverPort, languageAnalysisWireCodec, new LanguageAnalysisProviderWorker(providers)));
  resources.add(new LanguageAnalysisProviderModuleWireServer(serverPort, modules, host));
  resources.add(new TextMateGrammarCatalogWireServer(serverPort, grammarStore));
  const catalogs = resources.add(new TextMateGrammarCatalogModel(grammarCatalog(1, "keyword.control.demo")));
  const worker = resources.add(new TextMateAnalysisModuleWorkerClient(clientPort, catalogs, {
    requiredProviderModules: ["textmate.grammars", "test.fallback"],
  }));
  const localProviders = resources.add(new LanguageAnalysisProviderRegistry());
  const model = resources.add(new TextModel("if"));
  const analysis = resources.add(new LanguageAnalysisService(model, localProviders, { workerFactory: () => worker }));

  assert.equal((await analysis.requestTokens("demo")).status, LanguageRequestStatus.Applied);
  assert.equal(analysis.tokens.result!.value.tokens[0]!.tokenType, "keyword");
  assert.equal((await analysis.requestTokens("plain")).status, LanguageRequestStatus.Applied);
  assert.equal(analysis.tokens.result!.value.tokens[0]!.tokenType, "fallback");

  catalogs.replace(grammarCatalog(2, "string.quoted.demo"));
  assert.equal((await analysis.requestTokens("demo")).status, LanguageRequestStatus.Applied);
  assert.equal(analysis.tokens.result!.value.tokens[0]!.tokenType, "string");
  const catalogRequests = clientPort.sentMessages.filter(message => (message as { protocol?: string }).protocol === "zeta.textmate.grammar-catalog");
  assert.equal(catalogRequests.length, 2);
});

interface MemoryWirePort extends LanguageWorkerWireClientPort {
  readonly sentMessages: unknown[];
  connect(peer: MemoryWirePort): void;
}

function createPortPair(): readonly [MemoryWirePort, MemoryWirePort] {
  const first = new TestWirePort();
  const second = new TestWirePort();
  first.connect(second);
  second.connect(first);
  return [first, second];
}

class TestWirePort extends DisposableOwner implements MemoryWirePort {
  private readonly messageEmitter = this.own(new Emitter<unknown>());
  private readonly failureEmitter = this.own(new Emitter<unknown>());
  private peer: TestWirePort | undefined;
  private disposed = false;

  readonly sentMessages: unknown[] = [];
  readonly onMessage: Event<unknown> = this.messageEmitter.event;
  readonly onFailure: Event<unknown> = this.failureEmitter.event;

  constructor() {
    super();
    this.defer(() => {
      this.disposed = true;
      this.peer = undefined;
    });
  }

  connect(peer: TestWirePort): void {
    this.peer = peer;
  }

  send(message: unknown): void {
    if (this.disposed || !this.peer) throw new ReferenceError("Test wire port is unavailable");
    const cloned = structuredClone(message);
    const peer = this.peer;
    this.sentMessages.push(cloned);
    queueMicrotask(() => {
      if (!peer.disposed) peer.messageEmitter.fire(cloned);
    });
  }
}

function grammarCatalog(revision: number, keywordScope = "keyword.control.demo"): TextMateGrammarCatalog {
  return {
    revision,
    grammars: [{
      scopeName: "source.demo",
      languageId: "demo",
      injectTo: [],
      content: demoGrammar(keywordScope),
    }],
  };
}

function demoGrammar(keywordScope: string): string {
  return JSON.stringify({
    scopeName: "source.demo",
    patterns: [{ match: "\\bif\\b", name: keywordScope }],
    repository: {},
  });
}

async function initializeOnigLib(): Promise<IOnigLib> {
  const mainUrl = import.meta.resolve("vscode-oniguruma");
  const bytes = await readFile(new URL("onig.wasm", mainUrl));
  const data = bytes.buffer.slice(bytes.byteOffset, bytes.byteOffset + bytes.byteLength);
  await loadWASM(data);
  return Object.freeze({ createOnigScanner, createOnigString });
}
