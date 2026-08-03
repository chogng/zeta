import { raceCancellation } from "../../../../base/common/cancellation.js";
import { Emitter, type Event } from "../../../../base/common/event.js";
import { DisposableOwner } from "../../../../base/common/lifecycle.js";
import { assertLanguageId } from "../../../../editor/alpha/language/common/languageId.js";
import { type TextMateGrammarRegistrySnapshot } from "./textMateGrammarRegistry.js";

export interface TextMateGrammarCatalogEntry {
  readonly scopeName: string;
  readonly languageId?: string;
  readonly injectTo: readonly string[];
  readonly content: string;
}

export interface TextMateGrammarCatalog {
  readonly revision: number;
  readonly grammars: readonly TextMateGrammarCatalogEntry[];
}

export interface TextMateGrammarCatalogSource {
  readonly currentCatalog: TextMateGrammarCatalog;
  readonly onDidChangeCatalog: Event<TextMateGrammarCatalog>;
}

/** Mutable renderer-side source for complete versioned grammar catalogs. */
export class TextMateGrammarCatalogModel extends DisposableOwner implements TextMateGrammarCatalogSource {
  private readonly changeEmitter = this.own(new Emitter<TextMateGrammarCatalog>());
  private catalog: TextMateGrammarCatalog;
  private disposed = false;

  readonly onDidChangeCatalog: Event<TextMateGrammarCatalog> = this.changeEmitter.event;

  constructor(initialCatalog: TextMateGrammarCatalog = EMPTY_TEXTMATE_GRAMMAR_CATALOG) {
    super();
    this.catalog = normalizeTextMateGrammarCatalog(initialCatalog);
    this.defer(() => {
      this.disposed = true;
    });
  }

  get currentCatalog(): TextMateGrammarCatalog {
    this.ensureAlive();
    return this.catalog;
  }

  replace(catalog: TextMateGrammarCatalog): void {
    this.ensureAlive();
    const normalized = normalizeTextMateGrammarCatalog(catalog);
    if (normalized.revision <= this.catalog.revision) {
      throw new RangeError("TextMate grammar catalog revision must increase");
    }
    this.catalog = normalized;
    this.changeEmitter.fire(normalized);
  }

  private ensureAlive(): void {
    if (this.disposed) throw new ReferenceError("TextMateGrammarCatalogModel is already disposed");
  }
}

export function normalizeTextMateGrammarCatalog(value: TextMateGrammarCatalog): TextMateGrammarCatalog {
  if (typeof value !== "object" || value === null) {
    throw new TypeError("TextMate grammar catalog must be an object");
  }
  if (!Number.isSafeInteger(value.revision) || value.revision < 0) {
    throw new RangeError("TextMate grammar catalog revision must be a non-negative safe integer");
  }
  if (!Array.isArray(value.grammars)) {
    throw new TypeError("TextMate grammar catalog must contain grammars");
  }
  if (value.grammars.length > MAX_GRAMMAR_COUNT) {
    throw new RangeError(`TextMate grammar catalog cannot exceed ${MAX_GRAMMAR_COUNT} grammars`);
  }
  if (value.revision === 0 && value.grammars.length !== 0) {
    throw new RangeError("TextMate grammar catalog revision zero must be empty");
  }
  const scopes = new Set<string>();
  const languages = new Set<string>();
  let totalLength = 0;
  const grammars = value.grammars.map(grammar => {
    if (typeof grammar !== "object" || grammar === null) {
      throw new TypeError("TextMate grammar catalog entry must be an object");
    }
    assertScopeName(grammar.scopeName, "TextMate grammar catalog scope");
    if (scopes.has(grammar.scopeName)) throw new RangeError(`Duplicate TextMate grammar scope '${grammar.scopeName}'`);
    scopes.add(grammar.scopeName);
    if (grammar.languageId !== undefined) {
      assertLanguageId(grammar.languageId);
      if (languages.has(grammar.languageId)) throw new RangeError(`Duplicate TextMate root language '${grammar.languageId}'`);
      languages.add(grammar.languageId);
    }
    if (!Array.isArray(grammar.injectTo)) {
      throw new TypeError("TextMate grammar catalog injection targets must be an array");
    }
    const injectTo = [...grammar.injectTo];
    for (const scopeName of injectTo) assertScopeName(scopeName, "TextMate grammar catalog injection target");
    if (new Set(injectTo).size !== injectTo.length) {
      throw new RangeError("TextMate grammar catalog injection targets must be unique");
    }
    if (typeof grammar.content !== "string" || grammar.content.length === 0) {
      throw new TypeError("TextMate grammar catalog content must not be empty");
    }
    if (grammar.content.length > MAX_GRAMMAR_LENGTH) {
      throw new RangeError(`TextMate grammar content cannot exceed ${MAX_GRAMMAR_LENGTH} UTF-16 units`);
    }
    totalLength += grammar.content.length;
    if (totalLength > MAX_CATALOG_LENGTH) {
      throw new RangeError(`TextMate grammar catalog cannot exceed ${MAX_CATALOG_LENGTH} UTF-16 units`);
    }
    return Object.freeze({
      scopeName: grammar.scopeName,
      ...(grammar.languageId === undefined ? {} : { languageId: grammar.languageId }),
      injectTo: Object.freeze(injectTo),
      content: grammar.content,
    });
  });
  return Object.freeze({ revision: value.revision, grammars: Object.freeze(grammars) });
}

export async function materializeTextMateGrammarCatalog(snapshot: TextMateGrammarRegistrySnapshot, revision: number, signal: AbortSignal): Promise<TextMateGrammarCatalog> {
  if (!snapshot || typeof snapshot !== "object" || !Array.isArray(snapshot.grammars)) {
    throw new TypeError("TextMate grammar materialization requires a registry snapshot");
  }
  if (!Number.isSafeInteger(revision) || revision <= 0) {
    throw new RangeError("Materialized TextMate grammar catalog revision must be a positive safe integer");
  }
  signal.throwIfAborted();
  const loading = Promise.all(snapshot.grammars.map(async definition => {
    const loaded = await definition.loadGrammar();
    signal.throwIfAborted();
    if (typeof loaded !== "string" && loaded.scopeName !== definition.scopeName) {
      throw new TypeError(`TextMate grammar '${definition.scopeName}' returned a different root scope`);
    }
    const content = typeof loaded === "string" ? loaded : JSON.stringify(loaded);
    return {
      scopeName: definition.scopeName,
      ...(definition.languageId === undefined ? {} : { languageId: definition.languageId }),
      injectTo: definition.injectTo,
      content,
    };
  }));
  const grammars = await raceCancellation(loading, signal, "TextMate grammar materialization was cancelled");
  return normalizeTextMateGrammarCatalog({ revision, grammars });
}

export const EMPTY_TEXTMATE_GRAMMAR_CATALOG: TextMateGrammarCatalog = Object.freeze({
  revision: 0,
  grammars: Object.freeze([]),
});

const MAX_GRAMMAR_COUNT = 256;
const MAX_GRAMMAR_LENGTH = 4 * 1024 * 1024;
const MAX_CATALOG_LENGTH = 32 * 1024 * 1024;

function assertScopeName(value: unknown, owner: string): asserts value is string {
  if (typeof value !== "string" || !/^[A-Za-z0-9][A-Za-z0-9._+-]*$/.test(value)) {
    throw new TypeError(`${owner} is invalid`);
  }
}
