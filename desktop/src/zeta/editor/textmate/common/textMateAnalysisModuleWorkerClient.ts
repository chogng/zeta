import { raceCancellation } from "../../../base/common/cancellation.js";
import { DisposableOwner } from "../../../base/common/lifecycle.js";
import { LanguageAnalysisModuleWorkerClient } from "../../alpha/common/languageAnalysisModuleWorkerClient.js";
import { type LanguageAnalysisRequest } from "../../alpha/common/languageAnalysisProviders.js";
import { type LanguageAnalysisLane, type LanguageAnalysisResult, type LanguageAnalysisWorker } from "../../alpha/common/languageAnalysisService.js";
import { type LanguageWorkerModelSynchronizer, type LanguageWorkerRequest, type LanguageWorkerResultDisposition, type LanguageWorkerResultSettler } from "../../alpha/common/languageRequestCoordinator.js";
import { type LanguageWorkerWireClientPort } from "../../alpha/common/languageWorkerWire.js";
import { type TextModelChange } from "../../alpha/common/text.js";
import { type TextMateGrammarCatalog, type TextMateGrammarCatalogSource } from "./textMateGrammarCatalog.js";
import { TextMateGrammarCatalogWireClient } from "./textMateGrammarCatalogWire.js";

export interface TextMateAnalysisModuleWorkerClientOptions {
  readonly requiredProviderModules?: readonly string[];
}

/** Analysis Worker client gated by the latest renderer-owned grammar catalog. */
export class TextMateAnalysisModuleWorkerClient extends DisposableOwner implements LanguageAnalysisWorker, LanguageWorkerModelSynchronizer, LanguageWorkerResultSettler {
  private readonly worker: LanguageAnalysisModuleWorkerClient;
  private readonly catalogClient: TextMateGrammarCatalogWireClient;
  private catalogTail: Promise<void>;

  constructor(
    port: LanguageWorkerWireClientPort,
    catalogs: TextMateGrammarCatalogSource,
    options: TextMateAnalysisModuleWorkerClientOptions = {},
  ) {
    super();
    if (!catalogs || typeof catalogs !== "object" || typeof catalogs.onDidChangeCatalog !== "function" || !("currentCatalog" in catalogs)) {
      throw new TypeError("TextMate Analysis Worker client requires a grammar catalog source");
    }
    this.worker = this.own(new LanguageAnalysisModuleWorkerClient(port, options));
    this.catalogClient = this.own(new TextMateGrammarCatalogWireClient(port, error => this.worker.invalidate(error)));
    this.catalogTail = this.pushCatalog(catalogs.currentCatalog);
    this.observeTail();
    this.own(catalogs.onDidChangeCatalog(catalog => {
      this.catalogTail = this.catalogTail.then(() => this.pushCatalog(catalog));
      this.observeTail();
    }));
  }

  async run(request: LanguageWorkerRequest<LanguageAnalysisLane, LanguageAnalysisRequest>, signal: AbortSignal): Promise<LanguageAnalysisResult> {
    await this.waitForCurrentCatalog(signal);
    return this.worker.run(request, signal);
  }

  synchronizeModel(change: TextModelChange): void {
    this.worker.synchronizeModel(change);
  }

  settleResult(requestId: number, disposition: LanguageWorkerResultDisposition): void {
    this.worker.settleResult(requestId, disposition);
  }

  private async waitForCurrentCatalog(signal: AbortSignal): Promise<void> {
    while (true) {
      const tail = this.catalogTail;
      await raceCancellation(tail, signal, "TextMate grammar catalog wait was cancelled");
      if (tail === this.catalogTail) return;
    }
  }

  private pushCatalog(catalog: TextMateGrammarCatalog): Promise<void> {
    return catalog.revision === 0 ? Promise.resolve() : this.catalogClient.replaceCatalog(catalog);
  }

  private observeTail(): void {
    void this.catalogTail.catch(() => undefined);
  }
}
