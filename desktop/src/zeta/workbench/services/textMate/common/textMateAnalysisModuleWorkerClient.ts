import { raceCancellation } from "../../../../base/common/cancellation.js";
import { DisposableOwner } from "../../../../base/common/lifecycle.js";
import { LanguageAnalysisModuleWorkerClient } from "../../../../editor/alpha/language/common/languageAnalysisModuleWorkerClient.js";
import { type LanguageAnalysisRequest } from "../../../../editor/alpha/language/common/languageAnalysisProviders.js";
import { type LanguageAnalysisLane, type LanguageAnalysisResult, type LanguageAnalysisWorker } from "../../../../editor/alpha/language/common/languageAnalysisService.js";
import { type LanguageWorkerModelSynchronizer, type LanguageWorkerRequest, type LanguageWorkerResultDisposition, type LanguageWorkerResultSettler } from "../../../../editor/alpha/language/common/languageRequestCoordinator.js";
import { type LanguageWorkerWireClientPort } from "../../../../editor/alpha/language/common/languageWorkerWire.js";
import { type TextModelChange } from "../../../../editor/alpha/common/text.js";
import { type TextMateGrammarCatalog, type TextMateGrammarCatalogSource } from "./textMateGrammarCatalog.js";
import { TextMateGrammarCatalogWireClient } from "./textMateGrammarCatalogWire.js";
import { type TextMateScopeTheme, type TextMateScopeThemeSource } from "./textMateScopeTheme.js";
import { TextMateScopeThemeWireClient } from "./textMateScopeThemeWire.js";

export interface TextMateAnalysisModuleWorkerClientOptions {
  readonly requiredProviderModules?: readonly string[];
  /** Optional renderer-owned semantic scope theme mirrored into this Worker. */
  readonly scopeTheme?: TextMateScopeThemeSource;
}

/** Analysis Worker client gated by the latest renderer-owned grammar catalog. */
export class TextMateAnalysisModuleWorkerClient extends DisposableOwner implements LanguageAnalysisWorker, LanguageWorkerModelSynchronizer, LanguageWorkerResultSettler {
  private readonly worker: LanguageAnalysisModuleWorkerClient;
  private readonly catalogClient: TextMateGrammarCatalogWireClient;
  private readonly themeClient: TextMateScopeThemeWireClient | undefined;
  private catalogTail: Promise<void>;
  private themeTail: Promise<void>;

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
    if (options.scopeTheme !== undefined && (!options.scopeTheme || typeof options.scopeTheme !== "object" || typeof options.scopeTheme.onDidChangeTheme !== "function" || !("currentTheme" in options.scopeTheme))) {
      throw new TypeError("TextMate Analysis Worker scope theme must be a theme source");
    }
    this.themeClient = options.scopeTheme === undefined
      ? undefined
      : this.own(new TextMateScopeThemeWireClient(port, error => this.worker.invalidate(error)));
    this.catalogTail = this.pushCatalog(catalogs.currentCatalog);
    this.themeTail = options.scopeTheme === undefined ? Promise.resolve() : this.pushTheme(options.scopeTheme.currentTheme);
    this.observeTail();
    this.own(catalogs.onDidChangeCatalog(catalog => {
      this.catalogTail = this.catalogTail.then(() => this.pushCatalog(catalog));
      this.observeTail();
    }));
    if (options.scopeTheme) {
      this.own(options.scopeTheme.onDidChangeTheme(theme => {
        this.themeTail = this.themeTail.then(() => this.pushTheme(theme));
        this.observeTail();
      }));
    }
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
      const catalogTail = this.catalogTail;
      const themeTail = this.themeTail;
      await raceCancellation(Promise.all([catalogTail, themeTail]).then(() => undefined), signal, "TextMate grammar catalog wait was cancelled");
      if (catalogTail === this.catalogTail && themeTail === this.themeTail) return;
    }
  }

  private pushCatalog(catalog: TextMateGrammarCatalog): Promise<void> {
    return catalog.revision === 0 ? Promise.resolve() : this.catalogClient.replaceCatalog(catalog);
  }

  private pushTheme(theme: TextMateScopeTheme): Promise<void> {
    return theme.revision === 0 ? Promise.resolve() : this.themeClient!.replaceTheme(theme);
  }

  private observeTail(): void {
    void this.catalogTail.catch(() => undefined);
    void this.themeTail.catch(() => undefined);
  }
}
