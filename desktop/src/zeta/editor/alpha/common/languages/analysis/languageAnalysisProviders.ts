import { DisposableOwner, toDisposable, type IDisposable } from "../../../../../base/common/lifecycle.js";
import { assertLanguageId, assertLanguageSelector } from "../languageId.js";
import { type LanguageDiagnosticResult, type LanguageTokenResult } from "../languageResults.js";
import { type LanguageWorkerDocumentSynchronization } from "../languageWorkerDocumentMirror.js";
import { type TextSnapshot } from "../../../common/core/text.js";

export interface LanguageAnalysisRequest {
  readonly languageId: string;
}

export interface LanguageAnalysisProviderRequest extends LanguageAnalysisRequest {
  readonly requestId: number;
  readonly snapshot: TextSnapshot;
}

export interface LanguageAnalysisProvider {
  readonly id: string;
  readonly languageIds: readonly string[];
  readonly tokenPriority?: number;
  provideTokens?(request: LanguageAnalysisProviderRequest, signal: AbortSignal): LanguageTokenResult | undefined | PromiseLike<LanguageTokenResult | undefined>;
  provideDiagnostics?(request: LanguageAnalysisProviderRequest, signal: AbortSignal): LanguageDiagnosticResult | undefined | PromiseLike<LanguageDiagnosticResult | undefined>;
  synchronizeDocument?(synchronization: LanguageWorkerDocumentSynchronization): void;
}

export interface RegisteredLanguageAnalysisProvider {
  readonly id: string;
  readonly languageIds: readonly string[];
  readonly tokenPriority: number;
  readonly provideTokens?: NonNullable<LanguageAnalysisProvider["provideTokens"]>;
  readonly provideDiagnostics?: NonNullable<LanguageAnalysisProvider["provideDiagnostics"]>;
  readonly synchronizeDocument?: NonNullable<LanguageAnalysisProvider["synchronizeDocument"]>;
}

/** Caller-owned registry for snapshot tokenization and diagnostic providers. */
export class LanguageAnalysisProviderRegistry extends DisposableOwner {
  private readonly providers = new Map<string, RegisteredLanguageAnalysisProvider>();
  private disposed = false;

  constructor() {
    super();
    this.defer(() => {
      this.disposed = true;
      this.providers.clear();
    });
  }

  register(provider: LanguageAnalysisProvider): IDisposable {
    return this.registerMany([provider]);
  }

  registerMany(providers: readonly LanguageAnalysisProvider[]): IDisposable {
    this.ensureAlive();
    if (!Array.isArray(providers) || providers.length === 0) {
      throw new TypeError("Language analysis provider batch must not be empty");
    }
    const registered = providers.map(normalizeProvider);
    const identities = new Set<string>();
    for (const provider of registered) {
      if (identities.has(provider.id) || this.providers.has(provider.id)) {
        throw new RangeError(`Language analysis provider '${provider.id}' is already registered`);
      }
      identities.add(provider.id);
    }
    for (const provider of registered) this.providers.set(provider.id, provider);
    return toDisposable(() => {
      for (const provider of registered) {
        if (this.providers.get(provider.id) === provider) this.providers.delete(provider.id);
      }
    });
  }

  getTokenProvider(languageId: string): RegisteredLanguageAnalysisProvider | undefined {
    return this.getTokenProviders(languageId)[0];
  }

  getTokenProviders(languageId: string): readonly RegisteredLanguageAnalysisProvider[] {
    this.ensureAlive();
    assertLanguageId(languageId);
    const selected: RegisteredLanguageAnalysisProvider[] = [];
    for (const provider of this.providers.values()) {
      if (!provider.provideTokens || !matchesLanguage(provider, languageId)) continue;
      const index = selected.findIndex(candidate => provider.tokenPriority > candidate.tokenPriority);
      if (index < 0) selected.push(provider);
      else selected.splice(index, 0, provider);
    }
    return Object.freeze(selected);
  }

  getDiagnosticProviders(languageId: string): readonly RegisteredLanguageAnalysisProvider[] {
    this.ensureAlive();
    assertLanguageId(languageId);
    return Object.freeze([...this.providers.values()].filter(provider => provider.provideDiagnostics && matchesLanguage(provider, languageId)));
  }

  getDocumentSynchronizers(): readonly RegisteredLanguageAnalysisProvider[] {
    this.ensureAlive();
    return Object.freeze([...this.providers.values()].filter(provider => provider.synchronizeDocument));
  }

  private ensureAlive(): void {
    if (this.disposed) {
      throw new ReferenceError("LanguageAnalysisProviderRegistry is already disposed");
    }
  }
}

export function assertLanguageAnalysisRequest(request: LanguageAnalysisRequest): void {
  if (typeof request !== "object" || request === null) {
    throw new TypeError("Language analysis request must be an object");
  }
  assertLanguageId(request.languageId);
}

function normalizeProvider(provider: LanguageAnalysisProvider): RegisteredLanguageAnalysisProvider {
  if (typeof provider !== "object" || provider === null) {
    throw new TypeError("Language analysis provider must be an object");
  }
  assertIdentifier(provider.id, "Language analysis provider ID");
  if (!Array.isArray(provider.languageIds) || provider.languageIds.length === 0) {
    throw new TypeError("Language analysis provider must declare language IDs");
  }
  const languageIds = provider.languageIds.map(languageId => {
    assertLanguageSelector(languageId);
    return languageId;
  });
  if (new Set(languageIds).size !== languageIds.length) {
    throw new RangeError("Language analysis provider language IDs must be unique");
  }
  if (provider.provideTokens !== undefined && typeof provider.provideTokens !== "function") {
    throw new TypeError("Language analysis provider provideTokens must be a function");
  }
  if (provider.tokenPriority !== undefined && (!Number.isSafeInteger(provider.tokenPriority) || !provider.provideTokens)) {
    throw new TypeError("Language analysis provider token priority requires a token provider and must be a safe integer");
  }
  if (provider.provideDiagnostics !== undefined && typeof provider.provideDiagnostics !== "function") {
    throw new TypeError("Language analysis provider provideDiagnostics must be a function");
  }
  if (provider.synchronizeDocument !== undefined && typeof provider.synchronizeDocument !== "function") {
    throw new TypeError("Language analysis provider synchronizeDocument must be a function");
  }
  if (!provider.provideTokens && !provider.provideDiagnostics) {
    throw new TypeError("Language analysis provider must implement tokens or diagnostics");
  }
  return Object.freeze({
    id: provider.id,
    languageIds: Object.freeze(languageIds),
    tokenPriority: provider.tokenPriority ?? 0,
    ...(provider.provideTokens === undefined ? {} : { provideTokens: provider.provideTokens.bind(provider) }),
    ...(provider.provideDiagnostics === undefined ? {} : { provideDiagnostics: provider.provideDiagnostics.bind(provider) }),
    ...(provider.synchronizeDocument === undefined ? {} : { synchronizeDocument: provider.synchronizeDocument.bind(provider) }),
  });
}

function matchesLanguage(provider: RegisteredLanguageAnalysisProvider, languageId: string): boolean {
  return provider.languageIds.includes("*") || provider.languageIds.includes(languageId);
}

function assertIdentifier(value: unknown, owner: string): asserts value is string {
  if (typeof value !== "string" || !/^[A-Za-z0-9][A-Za-z0-9._-]*$/.test(value)) {
    throw new TypeError(`${owner} must contain only letters, digits, dot, underscore, or hyphen`);
  }
}
