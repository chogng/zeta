import { Emitter, type Event } from "../../../../base/common/event.js";
import { DisposableOwner, toDisposable, type IDisposable } from "../../../../base/common/lifecycle.js";
import { assertLanguageId } from "../../../../editor/alpha/language/common/languageId.js";
import { type IRawGrammar } from "vscode-textmate";

export type TextMateGrammarContent = string | IRawGrammar;
export type TextMateGrammarLoader = () => TextMateGrammarContent | PromiseLike<TextMateGrammarContent>;

export interface TextMateGrammarDefinition {
  readonly scopeName: string;
  readonly languageId?: string;
  readonly injectTo?: readonly string[];
  loadGrammar(): TextMateGrammarContent | PromiseLike<TextMateGrammarContent>;
}

export interface RegisteredTextMateGrammarDefinition {
  readonly scopeName: string;
  readonly languageId?: string;
  readonly injectTo: readonly string[];
  loadGrammar(): TextMateGrammarContent | PromiseLike<TextMateGrammarContent>;
}

export interface TextMateGrammarRegistrySnapshot {
  readonly revision: number;
  readonly languageIds: readonly string[];
  readonly grammars: readonly RegisteredTextMateGrammarDefinition[];
  getDefinitionForLanguage(languageId: string): RegisteredTextMateGrammarDefinition | undefined;
  getDefinitionForScope(scopeName: string): RegisteredTextMateGrammarDefinition | undefined;
  getInjections(scopeName: string): readonly string[];
}

/** Caller-owned TextMate grammar contributions with immutable revision snapshots. */
export class TextMateGrammarRegistry extends DisposableOwner {
  private readonly changeEmitter = this.own(new Emitter<TextMateGrammarRegistrySnapshot>());
  private readonly definitions = new Map<string, RegisteredTextMateGrammarDefinition>();
  private snapshot: TextMateGrammarRegistrySnapshot = createSnapshot(0, []);
  private disposed = false;

  readonly onDidChange: Event<TextMateGrammarRegistrySnapshot> = this.changeEmitter.event;

  constructor() {
    super();
    this.defer(() => {
      const changed = this.definitions.size > 0;
      this.definitions.clear();
      if (changed) this.publish();
      this.disposed = true;
    });
  }

  get currentSnapshot(): TextMateGrammarRegistrySnapshot {
    this.ensureAlive();
    return this.snapshot;
  }

  register(definition: TextMateGrammarDefinition): IDisposable {
    this.ensureAlive();
    const registered = normalizeDefinition(definition);
    if (this.definitions.has(registered.scopeName)) {
      throw new RangeError(`TextMate grammar scope '${registered.scopeName}' is already registered`);
    }
    if (registered.languageId && [...this.definitions.values()].some(value => value.languageId === registered.languageId)) {
      throw new RangeError(`TextMate language '${registered.languageId}' already has a root grammar`);
    }
    this.definitions.set(registered.scopeName, registered);
    this.publish();
    return toDisposable(() => {
      if (this.definitions.get(registered.scopeName) !== registered) return;
      this.definitions.delete(registered.scopeName);
      this.publish();
    });
  }

  private publish(): void {
    this.snapshot = createSnapshot(this.snapshot.revision + 1, [...this.definitions.values()]);
    this.changeEmitter.fire(this.snapshot);
  }

  private ensureAlive(): void {
    if (this.disposed) throw new ReferenceError("TextMateGrammarRegistry is already disposed");
  }
}

function normalizeDefinition(definition: TextMateGrammarDefinition): RegisteredTextMateGrammarDefinition {
  if (typeof definition !== "object" || definition === null) {
    throw new TypeError("TextMate grammar definition must be an object");
  }
  assertScopeName(definition.scopeName, "TextMate grammar scope");
  if (definition.languageId !== undefined) assertLanguageId(definition.languageId);
  if (!Array.isArray(definition.injectTo ?? [])) {
    throw new TypeError("TextMate grammar injection targets must be an array");
  }
  const injectTo = [...(definition.injectTo ?? [])];
  for (const scopeName of injectTo) assertScopeName(scopeName, "TextMate grammar injection target");
  if (new Set(injectTo).size !== injectTo.length) {
    throw new RangeError("TextMate grammar injection targets must be unique");
  }
  if (typeof definition.loadGrammar !== "function") {
    throw new TypeError("TextMate grammar definition must implement loadGrammar");
  }
  return Object.freeze({
    scopeName: definition.scopeName,
    ...(definition.languageId === undefined ? {} : { languageId: definition.languageId }),
    injectTo: Object.freeze(injectTo),
    loadGrammar: definition.loadGrammar.bind(definition),
  });
}

function createSnapshot(revision: number, values: readonly RegisteredTextMateGrammarDefinition[]): TextMateGrammarRegistrySnapshot {
  const grammars = Object.freeze([...values]);
  const definitionsByScope = new Map(grammars.map(definition => [definition.scopeName, definition]));
  const definitionsByLanguage = new Map(grammars.flatMap(definition => definition.languageId ? [[definition.languageId, definition] as const] : []));
  const injections = new Map<string, string[]>();
  for (const definition of values) {
    for (const scopeName of definition.injectTo) {
      const scopes = injections.get(scopeName) ?? [];
      scopes.push(definition.scopeName);
      injections.set(scopeName, scopes);
    }
  }
  const languageIds = Object.freeze([...definitionsByLanguage.keys()]);
  return Object.freeze({
    revision,
    languageIds,
    grammars,
    getDefinitionForLanguage: (languageId: string) => {
      assertLanguageId(languageId);
      return definitionsByLanguage.get(languageId);
    },
    getDefinitionForScope: (scopeName: string) => {
      assertScopeName(scopeName, "TextMate grammar scope");
      return definitionsByScope.get(scopeName);
    },
    getInjections: (scopeName: string) => {
      assertScopeName(scopeName, "TextMate grammar scope");
      return Object.freeze([...(injections.get(scopeName) ?? [])]);
    },
  });
}

function assertScopeName(value: unknown, owner: string): asserts value is string {
  if (typeof value !== "string" || !/^[A-Za-z0-9][A-Za-z0-9._+-]*$/.test(value)) {
    throw new TypeError(`${owner} must contain only letters, digits, dot, underscore, plus, or hyphen`);
  }
}
