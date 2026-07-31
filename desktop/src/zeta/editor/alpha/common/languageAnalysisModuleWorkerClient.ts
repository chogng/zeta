import { raceCancellation } from "../../../base/common/cancellation.js";
import { type Event } from "../../../base/common/event.js";
import { DisposableOwner } from "../../../base/common/lifecycle.js";
import { LanguageAnalysisProviderModuleState, type LanguageAnalysisProviderModuleCatalog, type LanguageAnalysisProviderModuleController, type LanguageAnalysisProviderModuleStateChange } from "./languageAnalysisProviderModules.js";
import { LanguageAnalysisProviderModuleWireClient } from "./languageAnalysisProviderModuleWire.js";
import { type LanguageAnalysisRequest } from "./languageAnalysisProviders.js";
import { type LanguageAnalysisLane, type LanguageAnalysisResult, type LanguageAnalysisWorker } from "./languageAnalysisService.js";
import { activateRequiredLanguageProviderModules, normalizeRequiredLanguageProviderModules } from "./languageProviderModules.js";
import { type LanguageWorkerModelSynchronizer, type LanguageWorkerRequest, type LanguageWorkerResultSettler, type LanguageWorkerResultDisposition } from "./languageRequestCoordinator.js";
import { languageAnalysisWireCodec } from "./languageAnalysisWire.js";
import { LanguageWorkerWireClient, type LanguageWorkerWireClientPort } from "./languageWorkerWire.js";
import { type TextModelChange } from "./text.js";

export interface LanguageAnalysisModuleWorkerClientOptions {
  readonly requiredProviderModules?: readonly string[];
}

/** Analysis Worker client with named provider-module activation readiness. */
export class LanguageAnalysisModuleWorkerClient extends DisposableOwner implements LanguageAnalysisWorker, LanguageWorkerModelSynchronizer, LanguageWorkerResultSettler, LanguageAnalysisProviderModuleController {
  private readonly worker: LanguageWorkerWireClient<LanguageAnalysisLane, LanguageAnalysisRequest, LanguageAnalysisResult>;
  private readonly modules: LanguageAnalysisProviderModuleWireClient;
  private readonly moduleReadiness: Promise<void>;

  readonly onDidChangeModuleCatalog: Event<LanguageAnalysisProviderModuleCatalog>;

  constructor(port: LanguageWorkerWireClientPort, options: LanguageAnalysisModuleWorkerClientOptions = {}) {
    super();
    const requiredProviderModules = normalizeRequiredLanguageProviderModules(options.requiredProviderModules);
    this.worker = this.own(new LanguageWorkerWireClient(port, languageAnalysisWireCodec));
    this.modules = this.own(new LanguageAnalysisProviderModuleWireClient(port, error => this.worker.invalidate(error)));
    this.onDidChangeModuleCatalog = this.modules.onDidChangeModuleCatalog;
    this.own(this.worker.onDidFail(error => this.modules.invalidate(error)));
    this.moduleReadiness = this.activateRequiredModules(requiredProviderModules);
    void this.moduleReadiness.catch(() => undefined);
  }

  get moduleCatalog(): LanguageAnalysisProviderModuleCatalog {
    return this.modules.moduleCatalog;
  }

  get moduleCatalogReady(): boolean {
    return this.modules.moduleCatalogReady;
  }

  waitForModuleCatalog(): Promise<LanguageAnalysisProviderModuleCatalog> {
    return this.modules.waitForModuleCatalog();
  }

  setProviderModuleActivation(moduleId: string, state: LanguageAnalysisProviderModuleState): Promise<LanguageAnalysisProviderModuleStateChange> {
    return this.modules.setProviderModuleActivation(moduleId, state);
  }

  async run(request: LanguageWorkerRequest<LanguageAnalysisLane, LanguageAnalysisRequest>, signal: AbortSignal): Promise<LanguageAnalysisResult> {
    await raceCancellation(this.moduleReadiness, signal, "Language analysis provider module wait was cancelled");
    return this.worker.run(request, signal);
  }

  synchronizeModel(change: TextModelChange): void {
    this.worker.synchronizeModel(change);
  }

  settleResult(requestId: number, disposition: LanguageWorkerResultDisposition): void {
    this.worker.settleResult(requestId, disposition);
  }

  invalidate(error: unknown): void {
    this.worker.invalidate(error);
  }

  private async activateRequiredModules(moduleIds: readonly string[]): Promise<void> {
    try {
      await activateRequiredLanguageProviderModules(this.modules, moduleIds);
    } catch (error) {
      try {
        this.worker.invalidate(error);
      } catch {
        // The required-module failure remains authoritative.
      }
      throw error;
    }
  }
}
