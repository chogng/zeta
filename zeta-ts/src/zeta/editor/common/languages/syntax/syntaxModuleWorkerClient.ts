import { raceCancellationError } from "../../../../base/common/async.js";
import { type Event } from "../../../../base/common/event.js";
import { Disposable } from "../../../../base/common/lifecycle.js";
import { SyntaxProviderModuleState, type SyntaxProviderModuleCatalog, type SyntaxProviderModuleController, type SyntaxProviderModuleStateChange } from "./syntaxProviderModules.js";
import { SyntaxProviderModuleWireClient } from "./syntaxProviderModuleWire.js";
import { type SyntaxRequest } from "./syntaxProviders.js";
import { type SyntaxLane, type SyntaxResult, type SyntaxWorker } from "./syntaxService.js";
import { activateRequiredLanguageProviderModules, normalizeRequiredLanguageProviderModules } from "../languageProviderModules.js";
import { type LanguageWorkerModelSynchronizer, type LanguageWorkerRequest, type LanguageWorkerResultSettler, type LanguageWorkerResultDisposition } from "../languageRequestCoordinator.js";
import { syntaxWireCodec } from "./syntaxWire.js";
import { LanguageWorkerWireClient, type LanguageWorkerWireClientPort } from "../languageWorkerWire.js";
import { type TextModelChange } from "../../core/textChange.js";

export interface SyntaxModuleWorkerClientOptions {
	readonly requiredProviderModules?: readonly string[];
}

/** Syntax Worker client with named provider-module activation readiness. */
export class SyntaxModuleWorkerClient extends Disposable implements SyntaxWorker, LanguageWorkerModelSynchronizer, LanguageWorkerResultSettler, SyntaxProviderModuleController {
	private readonly worker: LanguageWorkerWireClient<SyntaxLane, SyntaxRequest, SyntaxResult>;
	private readonly modules: SyntaxProviderModuleWireClient;
	private readonly moduleReadiness: Promise<void>;

	readonly onDidChangeModuleCatalog: Event<SyntaxProviderModuleCatalog>;

	constructor(port: LanguageWorkerWireClientPort, options: SyntaxModuleWorkerClientOptions = {}) {
		super();
		const requiredProviderModules = normalizeRequiredLanguageProviderModules(options.requiredProviderModules);
		this.worker = this._register(new LanguageWorkerWireClient(port, syntaxWireCodec));
		this.modules = this._register(new SyntaxProviderModuleWireClient(port, error => this.worker.invalidate(error)));
		this.onDidChangeModuleCatalog = this.modules.onDidChangeModuleCatalog;
		this._register(this.worker.onDidFail(error => this.modules.invalidate(error)));
		this.moduleReadiness = this.activateRequiredModules(requiredProviderModules);
		void this.moduleReadiness.catch(() => undefined);
	}

	get moduleCatalog(): SyntaxProviderModuleCatalog {
		return this.modules.moduleCatalog;
	}

	get moduleCatalogReady(): boolean {
		return this.modules.moduleCatalogReady;
	}

	waitForModuleCatalog(): Promise<SyntaxProviderModuleCatalog> {
		return this.modules.waitForModuleCatalog();
	}

	setProviderModuleActivation(moduleId: string, state: SyntaxProviderModuleState): Promise<SyntaxProviderModuleStateChange> {
		return this.modules.setProviderModuleActivation(moduleId, state);
	}

	async run(request: LanguageWorkerRequest<SyntaxLane, SyntaxRequest>, signal: AbortSignal): Promise<SyntaxResult> {
		await raceCancellationError(this.moduleReadiness, signal, "Syntax provider module wait was cancelled");
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
