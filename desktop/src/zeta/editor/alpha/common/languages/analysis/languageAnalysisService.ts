import { DisposableOwner } from "../../../../../base/common/lifecycle.js";
import { assertLanguageAnalysisRequest, LanguageAnalysisProviderRegistry, type LanguageAnalysisProviderRequest, type LanguageAnalysisRequest, type RegisteredLanguageAnalysisProvider } from "./languageAnalysisProviders.js";
import { LanguageRequestCoordinator, type LanguageRequestOptions, type LanguageRequestOutcome, type LanguageWorker, type LanguageWorkerRequest } from "../languageRequestCoordinator.js";
import { LanguageResultAcceptance } from "../languageResultStore.js";
import { createLanguageDiagnosticSnapshotNormalizer, createLanguageDiagnosticStore, createLanguageTokenSnapshotNormalizer, createLanguageTokenStore, type LanguageDiagnostic, type LanguageDiagnosticResult, type LanguageTokenResult } from "../languageResults.js";
import { type TextModel } from "../../../common/model/textModel.js";
import { type LanguageWorkerDocumentSynchronization, type LanguageWorkerDocumentSynchronizationObserver } from "../languageWorkerDocumentMirror.js";

export const LANGUAGE_TOKEN_LANE = "tokens";
export const LANGUAGE_DIAGNOSTIC_LANE = "diagnostics";
export const LANGUAGE_ANALYSIS_SYNCHRONIZATION = "synchronization";
export type LanguageAnalysisLane = typeof LANGUAGE_TOKEN_LANE | typeof LANGUAGE_DIAGNOSTIC_LANE;
export type LanguageAnalysisProviderOperation = LanguageAnalysisLane | typeof LANGUAGE_ANALYSIS_SYNCHRONIZATION;
export type LanguageAnalysisWorker = LanguageWorker<LanguageAnalysisLane, LanguageAnalysisRequest, LanguageAnalysisResult>;
export type LanguageAnalysisWorkerFactory = () => LanguageAnalysisWorker;
export type LanguageAnalysisProviderErrorHandler = (providerId: string, operation: LanguageAnalysisProviderOperation, error: unknown) => void;

export interface LanguageTokenAnalysisResult {
  readonly lane: typeof LANGUAGE_TOKEN_LANE;
  readonly value: LanguageTokenResult;
}

export interface LanguageDiagnosticAnalysisResult {
  readonly lane: typeof LANGUAGE_DIAGNOSTIC_LANE;
  readonly value: LanguageDiagnosticResult;
}

export type LanguageAnalysisResult = LanguageTokenAnalysisResult | LanguageDiagnosticAnalysisResult;

export interface LanguageAnalysisServiceOptions {
  readonly workerFactory?: LanguageAnalysisWorkerFactory;
  readonly onProviderError?: LanguageAnalysisProviderErrorHandler;
}

export interface LanguageAnalysisRequestOutcomes {
  readonly tokens: LanguageRequestOutcome;
  readonly diagnostics: LanguageRequestOutcome;
}

/** Runs token and diagnostic lanes over one reusable snapshot worker. */
export class LanguageAnalysisService extends DisposableOwner {
  readonly tokens: ReturnType<typeof createLanguageTokenStore>;
  readonly diagnostics: ReturnType<typeof createLanguageDiagnosticStore>;
  private readonly coordinator: LanguageRequestCoordinator<LanguageAnalysisLane, LanguageAnalysisRequest, LanguageAnalysisResult>;

  constructor(
    model: TextModel,
    registry: LanguageAnalysisProviderRegistry,
    options: LanguageAnalysisServiceOptions = {},
  ) {
    super();
    if (!(registry instanceof LanguageAnalysisProviderRegistry)) {
      this.dispose();
      throw new TypeError("Language analysis service requires a provider registry");
    }
    if (options.workerFactory !== undefined && typeof options.workerFactory !== "function") {
      this.dispose();
      throw new TypeError("Language analysis worker factory must be a function");
    }
    if (options.onProviderError !== undefined && typeof options.onProviderError !== "function") {
      this.dispose();
      throw new TypeError("Language analysis provider error handler must be a function");
    }
    if (options.workerFactory && options.onProviderError) {
      this.dispose();
      throw new TypeError("A custom language analysis worker owns its provider error policy");
    }
    this.tokens = this.own(createLanguageTokenStore(model));
    this.diagnostics = this.own(createLanguageDiagnosticStore(model));
    this.coordinator = this.own(new LanguageRequestCoordinator(
      model,
      options.workerFactory ?? (() => new LanguageAnalysisProviderWorker(registry, options.onProviderError)),
    ));
  }

  requestTokens(languageId: string, options: LanguageRequestOptions = {}): Promise<LanguageRequestOutcome> {
    const request = analysisRequest(languageId);
    return this.coordinator.runLatest(LANGUAGE_TOKEN_LANE, request, result => {
      if (result.value.lane !== LANGUAGE_TOKEN_LANE) {
        throw new TypeError(`Token lane received '${result.value.lane}'`);
      }
      const acceptance = this.tokens.accept(Object.freeze({
        ...result,
        value: result.value.value,
      }));
      assertApplied(acceptance, LANGUAGE_TOKEN_LANE);
    }, options);
  }

  requestDiagnostics(languageId: string, options: LanguageRequestOptions = {}): Promise<LanguageRequestOutcome> {
    const request = analysisRequest(languageId);
    return this.coordinator.runLatest(LANGUAGE_DIAGNOSTIC_LANE, request, result => {
      if (result.value.lane !== LANGUAGE_DIAGNOSTIC_LANE) {
        throw new TypeError(`Diagnostic lane received '${result.value.lane}'`);
      }
      const acceptance = this.diagnostics.accept(Object.freeze({
        ...result,
        value: result.value.value,
      }));
      assertApplied(acceptance, LANGUAGE_DIAGNOSTIC_LANE);
    }, options);
  }

  async requestAll(languageId: string, options: LanguageRequestOptions = {}): Promise<LanguageAnalysisRequestOutcomes> {
    const [tokens, diagnostics] = await Promise.all([
      this.requestTokens(languageId, options),
      this.requestDiagnostics(languageId, options),
    ]);
    return Object.freeze({ tokens, diagnostics });
  }
}

/** Provider host shared by in-process and Worker transports. */
export class LanguageAnalysisProviderWorker implements LanguageAnalysisWorker, LanguageWorkerDocumentSynchronizationObserver {
  private disposed = false;

  constructor(
    private readonly registry: LanguageAnalysisProviderRegistry,
    private readonly onProviderError: LanguageAnalysisProviderErrorHandler = reportProviderError,
  ) {
    if (typeof onProviderError !== "function") {
      throw new TypeError("Language analysis provider error handler must be a function");
    }
  }

  async run(request: LanguageWorkerRequest<LanguageAnalysisLane, LanguageAnalysisRequest>, signal: AbortSignal): Promise<LanguageAnalysisResult> {
    this.ensureAlive();
    signal.throwIfAborted();
    assertLanguageAnalysisRequest(request.payload);
    if (request.lane === LANGUAGE_TOKEN_LANE) {
      return Object.freeze({
        lane: LANGUAGE_TOKEN_LANE,
        value: await this.runTokens(request, signal),
      });
    }
    if (request.lane === LANGUAGE_DIAGNOSTIC_LANE) {
      return Object.freeze({
        lane: LANGUAGE_DIAGNOSTIC_LANE,
        value: await this.runDiagnostics(request, signal),
      });
    }
    throw new RangeError(`Unknown language analysis lane '${request.lane}'`);
  }

  dispose(): void {
    this.disposed = true;
  }

  [Symbol.dispose](): void {
    this.dispose();
  }

  synchronizeDocument(synchronization: LanguageWorkerDocumentSynchronization): void {
    this.ensureAlive();
    for (const provider of this.registry.getDocumentSynchronizers()) {
      try {
        provider.synchronizeDocument!(synchronization);
      } catch (error) {
        this.reportProviderError(provider.id, LANGUAGE_ANALYSIS_SYNCHRONIZATION, error);
      }
    }
  }

  private async runTokens(request: LanguageWorkerRequest<LanguageAnalysisLane, LanguageAnalysisRequest>, signal: AbortSignal): Promise<LanguageTokenResult> {
    const providers = this.registry.getTokenProviders(request.payload.languageId);
    if (providers.length === 0) return EMPTY_TOKENS;
    const normalize = createLanguageTokenSnapshotNormalizer(request.snapshot);
    for (const provider of providers) {
      try {
        const value = await provider.provideTokens!(providerRequest(request), signal);
        signal.throwIfAborted();
        if (value !== undefined) return normalize(value);
      } catch (error) {
        if (signal.aborted) throw error;
        this.reportProviderError(provider.id, LANGUAGE_TOKEN_LANE, error);
      }
    }
    return EMPTY_TOKENS;
  }

  private async runDiagnostics(request: LanguageWorkerRequest<LanguageAnalysisLane, LanguageAnalysisRequest>, signal: AbortSignal): Promise<LanguageDiagnosticResult> {
    const providers = this.registry.getDiagnosticProviders(request.payload.languageId);
    if (providers.length === 0) return EMPTY_DIAGNOSTICS;
    const normalize = createLanguageDiagnosticSnapshotNormalizer(request.snapshot);
    const batches = await Promise.all(providers.map(provider => this.runDiagnosticProvider(provider, request, signal, normalize)));
    signal.throwIfAborted();
    const diagnostics: LanguageDiagnostic[] = [];
    for (const batch of batches) diagnostics.push(...(batch?.diagnostics ?? []));
    return normalize({ diagnostics });
  }

  private async runDiagnosticProvider(
    provider: RegisteredLanguageAnalysisProvider,
    request: LanguageWorkerRequest<LanguageAnalysisLane, LanguageAnalysisRequest>,
    signal: AbortSignal,
    normalize: (value: LanguageDiagnosticResult) => LanguageDiagnosticResult,
  ): Promise<LanguageDiagnosticResult | undefined> {
    try {
      const value = await provider.provideDiagnostics!(providerRequest(request), signal);
      signal.throwIfAborted();
      return value === undefined ? undefined : normalize(value);
    } catch (error) {
      if (signal.aborted) throw error;
      this.reportProviderError(provider.id, LANGUAGE_DIAGNOSTIC_LANE, error);
      return undefined;
    }
  }

  private reportProviderError(providerId: string, operation: LanguageAnalysisProviderOperation, error: unknown): void {
    try {
      this.onProviderError(providerId, operation, error);
    } catch (reportingError) {
      reportProviderError(providerId, operation, new AggregateError([error, reportingError], "Language analysis and error reporting both failed"));
    }
  }

  private ensureAlive(): void {
    if (this.disposed) {
      throw new ReferenceError("LanguageAnalysisProviderWorker is already disposed");
    }
  }
}

const EMPTY_TOKENS: LanguageTokenResult = Object.freeze({ tokens: Object.freeze([]) });
const EMPTY_DIAGNOSTICS: LanguageDiagnosticResult = Object.freeze({ diagnostics: Object.freeze([]) });

function analysisRequest(languageId: string): LanguageAnalysisRequest {
  const request = Object.freeze({ languageId });
  assertLanguageAnalysisRequest(request);
  return request;
}

function providerRequest(request: LanguageWorkerRequest<LanguageAnalysisLane, LanguageAnalysisRequest>): LanguageAnalysisProviderRequest {
  return Object.freeze({
    requestId: request.requestId,
    snapshot: request.snapshot,
    languageId: request.payload.languageId,
  });
}

function assertApplied(acceptance: LanguageResultAcceptance, lane: LanguageAnalysisLane): void {
  if (acceptance !== LanguageResultAcceptance.Applied) {
    throw new Error(`Language ${lane} store rejected current result as '${acceptance}'`);
  }
}

function reportProviderError(providerId: string, operation: LanguageAnalysisProviderOperation, error: unknown): void {
  console.error(`Language analysis provider '${providerId}' failed in '${operation}'`, error);
}
