import { strict as assert } from "node:assert";
import test from "node:test";
import { Emitter, type Event } from "../../../../base/common/event.js";
import { DisposableOwner, DisposableStore } from "../../../../base/common/lifecycle.js";
import { LanguageAnalysisModuleWorkerClient } from "../../common/languages/analysis/languageAnalysisModuleWorkerClient.js";
import { LanguageAnalysisProviderRegistry } from "../../common/languages/analysis/languageAnalysisProviders.js";
import { LanguageAnalysisProviderModuleHost, LanguageAnalysisProviderModuleRegistry } from "../../common/languages/analysis/languageAnalysisProviderModules.js";
import { LanguageAnalysisProviderModuleWireServer } from "../../common/languages/analysis/languageAnalysisProviderModuleWire.js";
import { LANGUAGE_TOKEN_LANE, LanguageAnalysisProviderWorker, LanguageAnalysisService } from "../../common/languages/analysis/languageAnalysisService.js";
import { languageAnalysisWireCodec } from "../../common/languages/analysis/languageAnalysisWire.js";
import { createLanguageLexicalAnalysisProvider } from "../../common/languages/languageLexicalAnalysisProvider.js";
import { LanguageRequestStatus } from "../../common/languages/languageRequestCoordinator.js";
import { LanguageWorkerWireServer, type LanguageWorkerWireClientPort } from "../../common/languages/languageWorkerWire.js";
import { TextPosition, TextRange } from "../../common/core/text.js";
import { TextModel } from "../../common/model/textModel.js";

test("Required Analysis modules activate before the first request and preserve confirmed result bases", async () => {
  using model = new TextModel("const value = 1;");
  using providers = new LanguageAnalysisProviderRegistry();
  using modules = new LanguageAnalysisProviderModuleRegistry();
  using moduleRegistration = modules.register({
    id: "language.lexical",
    load: async () => {
      await new Promise<void>(resolve => setImmediate(resolve));
      return [createLanguageLexicalAnalysisProvider()];
    },
  });
  using host = new LanguageAnalysisProviderModuleHost(modules, providers);
  const [clientPort, serverPort] = createPortPair();
  using workerServer = new LanguageWorkerWireServer(serverPort, languageAnalysisWireCodec, new LanguageAnalysisProviderWorker(providers));
  using moduleServer = new LanguageAnalysisProviderModuleWireServer(serverPort, modules, host);
  using localProviders = new LanguageAnalysisProviderRegistry();
  using service = new LanguageAnalysisService(model, localProviders, {
    workerFactory: () => new LanguageAnalysisModuleWorkerClient(clientPort, {
      requiredProviderModules: ["language.lexical"],
    }),
  });

  assert.equal((await service.requestTokens("typescript")).status, LanguageRequestStatus.Applied);
  assert.deepEqual(service.tokens.result!.value.tokens.map(token => token.tokenType), ["keyword", "variable", "operator", "number"]);
  const firstMessages = clientPort.sentMessages as WireMessage[];
  const activationIndex = firstMessages.findIndex(message => message.protocol === "zeta.language.analysis-provider-modules" && message.kind === "setActivation");
  const requestIndex = firstMessages.findIndex(message => message.protocol === "zeta.language-worker" && message.kind === "request");
  assert.equal(activationIndex >= 0, true);
  assert.equal(requestIndex > activationIndex, true);

  model.applyEdits([{
    range: TextRange.emptyAt(TextPosition.at(0, model.getText().length)),
    text: "\nreturn value;",
  }]);
  assert.equal((await service.requestTokens("typescript")).status, LanguageRequestStatus.Applied);
  const requests = (clientPort.sentMessages as WireMessage[]).filter(message => message.protocol === "zeta.language-worker" && message.kind === "request");
  assert.equal(requests[0]!.lane, LANGUAGE_TOKEN_LANE);
  assert.equal(requests[0]!.resultBaseRequestId, undefined);
  assert.equal(requests[1]!.resultBaseRequestId, 1);
});

test("Required Analysis module failure discards the Worker before the next request", async () => {
  using model = new TextModel("const value = 1;");
  using localProviders = new LanguageAnalysisProviderRegistry();
  using workerResources = new DisposableStore();
  let workerCount = 0;
  using service = new LanguageAnalysisService(model, localProviders, {
    workerFactory: () => {
      workerCount += 1;
      const providers = workerResources.add(new LanguageAnalysisProviderRegistry());
      const modules = workerResources.add(new LanguageAnalysisProviderModuleRegistry());
      workerResources.add(modules.register({
        id: "language.lexical",
        load: () => {
          if (workerCount === 1) throw new Error("analysis module failed");
          return [createLanguageLexicalAnalysisProvider()];
        },
      }));
      const host = workerResources.add(new LanguageAnalysisProviderModuleHost(modules, providers));
      const [clientPort, serverPort] = createPortPair();
      workerResources.add(new LanguageWorkerWireServer(serverPort, languageAnalysisWireCodec, new LanguageAnalysisProviderWorker(providers)));
      workerResources.add(new LanguageAnalysisProviderModuleWireServer(serverPort, modules, host));
      return new LanguageAnalysisModuleWorkerClient(clientPort, {
        requiredProviderModules: ["language.lexical"],
      });
    },
  });

  await assert.rejects(service.requestTokens("typescript"), /analysis module failed/);
  const outcome = await service.requestTokens("typescript");

  assert.equal(outcome.status, LanguageRequestStatus.Applied);
  assert.equal(workerCount, 2);
  assert.equal(service.tokens.result!.value.tokens[0]!.tokenType, "keyword");
});

interface WireMessage {
  readonly protocol?: string;
  readonly kind?: string;
  readonly lane?: string;
  readonly resultBaseRequestId?: number;
}

function createPortPair(): readonly [MemoryAnalysisModulePort, MemoryAnalysisModulePort] {
  const first = new MemoryAnalysisModulePort();
  const second = new MemoryAnalysisModulePort();
  first.connect(second);
  second.connect(first);
  return [first, second];
}

class MemoryAnalysisModulePort extends DisposableOwner implements LanguageWorkerWireClientPort {
  private readonly messageEmitter = this.own(new Emitter<unknown>());
  private readonly failureEmitter = this.own(new Emitter<unknown>());
  private peer: MemoryAnalysisModulePort | undefined;
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

  connect(peer: MemoryAnalysisModulePort): void {
    this.peer = peer;
  }

  send(message: unknown): void {
    if (this.disposed || !this.peer) {
      throw new ReferenceError("Memory analysis module port is unavailable");
    }
    const peer = this.peer;
    const cloned = structuredClone(message);
    this.sentMessages.push(cloned);
    queueMicrotask(() => {
      if (!peer.disposed) peer.messageEmitter.fire(cloned);
    });
  }
}
