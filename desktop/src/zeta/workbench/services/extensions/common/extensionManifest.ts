export interface ExtensionGrammarContribution {
  readonly language?: string;
  readonly scopeName: string;
  readonly path: string;
  readonly injectTo: readonly string[];
  readonly embeddedLanguages?: Readonly<Record<string, string>>;
  readonly tokenTypes?: Readonly<Record<string, "string" | "other" | "comment" | "regex">>;
  readonly balancedBracketScopes?: readonly string[];
  readonly unbalancedBracketScopes?: readonly string[];
}

export interface ExtensionLanguageContribution {
  readonly id: string;
  readonly aliases: readonly string[];
  readonly extensions: readonly string[];
  readonly filenames: readonly string[];
  readonly filenamePatterns: readonly string[];
  readonly mimetypes: readonly string[];
  readonly configuration?: string;
}

export interface ExtensionSnippetContribution {
  readonly language: readonly string[];
  readonly path: string;
}

export interface ExtensionThemeContribution {
  readonly label: string;
  readonly path: string;
  readonly uiTheme?: string;
}

export interface ExtensionManifest {
  readonly name: string;
  readonly publisher: string;
  readonly version: string;
  readonly displayName: string;
  readonly contributes: {
    readonly languages: readonly ExtensionLanguageContribution[];
    readonly grammars: readonly ExtensionGrammarContribution[];
    readonly snippets: readonly ExtensionSnippetContribution[];
    readonly themes: readonly ExtensionThemeContribution[];
  };
}

export interface ExtensionManifestDescriptor {
  readonly id: string;
  readonly name: string;
  readonly publisher: string;
  readonly version: string;
  readonly displayName?: string;
}

/** Parses and validates the declarative contribution subset owned by Workbench. */
export function parseExtensionManifest(manifestJson: string, descriptor: ExtensionManifestDescriptor): ExtensionManifest {
  let value: unknown;
  try {
    value = JSON.parse(manifestJson);
  } catch {
    throw new TypeError(`Extension '${descriptor.id}' manifest is not valid JSON`);
  }
  const manifest = record(value, `Extension '${descriptor.id}' manifest`);
  const name = requiredString(manifest.name, "name", 128);
  const publisher = requiredString(manifest.publisher, "publisher", 128);
  const version = requiredString(manifest.version, "version", 128);
  if (name !== descriptor.name || publisher !== descriptor.publisher || version !== descriptor.version) {
    throw new TypeError(`Extension '${descriptor.id}' manifest identity does not match its catalog entry`);
  }
  const displayName = typeof manifest.displayName === "string" && manifest.displayName.trim().length > 0
    ? boundedText(manifest.displayName, "displayName", 256)
    : descriptor.displayName ?? name;
  const contributes = manifest.contributes === undefined ? {} : record(manifest.contributes, "Extension contributes");
  return Object.freeze({
    name,
    publisher,
    version,
    displayName,
    contributes: Object.freeze({
      languages: Object.freeze(contributes.languages === undefined ? [] : parseLanguages(contributes.languages, descriptor.id)),
      grammars: Object.freeze(contributes.grammars === undefined ? [] : parseGrammars(contributes.grammars, descriptor.id)),
      snippets: Object.freeze(contributes.snippets === undefined ? [] : parseSnippets(contributes.snippets, descriptor.id)),
      themes: Object.freeze(contributes.themes === undefined ? [] : parseThemes(contributes.themes, descriptor.id)),
    }),
  });
}

function parseLanguages(value: unknown, extensionId: string): readonly ExtensionLanguageContribution[] {
  if (!Array.isArray(value)) throw new TypeError(`Extension '${extensionId}' language contributions must be an array`);
  return value.map((candidate, index) => {
    const language = record(candidate, `Extension '${extensionId}' language ${index}`);
    const id = languageId(language.id, `Extension '${extensionId}' language ${index} id`);
    return Object.freeze({
      id,
      aliases: parseTextList(language.aliases, `Extension '${extensionId}' language ${index} aliases`),
      extensions: parseExtensions(language.extensions, extensionId, index),
      filenames: parseTextList(language.filenames, `Extension '${extensionId}' language ${index} filenames`, true),
      filenamePatterns: parseTextList(language.filenamePatterns, `Extension '${extensionId}' language ${index} filename patterns`, true),
      mimetypes: parseTextList(language.mimetypes, `Extension '${extensionId}' language ${index} MIME types`, true),
      ...(language.configuration === undefined ? {} : { configuration: normalizeResourcePath(language.configuration, `Extension '${extensionId}' language ${index} configuration`) }),
    });
  });
}

function parseExtensions(value: unknown, extensionId: string, index: number): readonly string[] {
  const extensions = parseTextList(value, `Extension '${extensionId}' language ${index} extensions`, true, false);
  if (extensions.some(extension => !extension.startsWith("."))) {
    throw new TypeError(`Extension '${extensionId}' language ${index} extensions must start with a dot`);
  }
  return Object.freeze([...new Set(extensions)]);
}

function parseGrammars(value: unknown, extensionId: string): readonly ExtensionGrammarContribution[] {
  if (!Array.isArray(value)) throw new TypeError(`Extension '${extensionId}' grammar contributions must be an array`);
  return value.map((candidate, index) => {
    const grammar = record(candidate, `Extension '${extensionId}' grammar ${index}`);
    const scopeName = scopeNameValue(grammar.scopeName, `Extension '${extensionId}' grammar ${index} scopeName`);
    const language = grammar.language === undefined ? undefined : languageId(grammar.language, `Extension '${extensionId}' grammar ${index} language`);
    const injectTo = grammar.injectTo === undefined ? Object.freeze([]) : parseScopes(grammar.injectTo, `Extension '${extensionId}' grammar ${index} injectTo`);
    const embeddedLanguages = grammar.embeddedLanguages === undefined ? undefined : parseScopeMap(grammar.embeddedLanguages, extensionId, index, "embeddedLanguages", languageId);
    const tokenTypes = grammar.tokenTypes === undefined ? undefined : parseScopeMap(grammar.tokenTypes, extensionId, index, "tokenTypes", tokenTypeValue, selectorValue);
    const balancedBracketScopes = grammar.balancedBracketScopes === undefined ? undefined : parseBracketScopes(grammar.balancedBracketScopes, `Extension '${extensionId}' grammar ${index} balancedBracketScopes`);
    const unbalancedBracketScopes = grammar.unbalancedBracketScopes === undefined ? undefined : parseBracketScopes(grammar.unbalancedBracketScopes, `Extension '${extensionId}' grammar ${index} unbalancedBracketScopes`);
    return Object.freeze({
      ...(language === undefined ? {} : { language }),
      scopeName,
      path: normalizeResourcePath(grammar.path, `Extension '${extensionId}' grammar ${index} path`),
      injectTo,
      ...(embeddedLanguages === undefined ? {} : { embeddedLanguages }),
      ...(tokenTypes === undefined ? {} : { tokenTypes }),
      ...(balancedBracketScopes === undefined ? {} : { balancedBracketScopes }),
      ...(unbalancedBracketScopes === undefined ? {} : { unbalancedBracketScopes }),
    });
  });
}

function parseSnippets(value: unknown, extensionId: string): readonly ExtensionSnippetContribution[] {
  if (!Array.isArray(value)) throw new TypeError(`Extension '${extensionId}' snippet contributions must be an array`);
  return value.map((candidate, index) => {
    const snippet = record(candidate, `Extension '${extensionId}' snippet ${index}`);
    const language = snippet.language === undefined ? snippet.languageIds : snippet.language;
    const languages = typeof language === "string" ? [language] : parseTextList(language, `Extension '${extensionId}' snippet ${index} language`, false);
    if (languages.length === 0) throw new TypeError(`Extension '${extensionId}' snippet ${index} must declare a language`);
    return Object.freeze({
      language: Object.freeze(languages.map((value, languageIndex) => languageId(value, `Extension '${extensionId}' snippet ${index} language ${languageIndex}`))),
      path: normalizeResourcePath(snippet.path, `Extension '${extensionId}' snippet ${index} path`),
    });
  });
}

function parseThemes(value: unknown, extensionId: string): readonly ExtensionThemeContribution[] {
  if (!Array.isArray(value)) throw new TypeError(`Extension '${extensionId}' theme contributions must be an array`);
  return value.map((candidate, index) => {
    const theme = record(candidate, `Extension '${extensionId}' theme ${index}`);
    return Object.freeze({
      label: requiredString(theme.label, `Extension '${extensionId}' theme ${index} label`, 256),
      path: normalizeResourcePath(theme.path, `Extension '${extensionId}' theme ${index} path`),
      ...(theme.uiTheme === undefined ? {} : { uiTheme: boundedText(theme.uiTheme, `Extension '${extensionId}' theme ${index} uiTheme`, 128) }),
    });
  });
}

function parseScopeMap<T>(value: unknown, extensionId: string, index: number, field: string, parseValue: (value: unknown, owner: string, maximum?: number) => T, parseKey: (value: string, owner: string) => string = scopeNameValue): Readonly<Record<string, T>> {
  const object = record(value, `Extension '${extensionId}' grammar ${index} ${field}`);
  const entries = Object.entries(object).map(([scope, mapped]) => [
    parseKey(scope, `Extension '${extensionId}' grammar ${index} ${field} scope`),
    parseValue(mapped, `Extension '${extensionId}' grammar ${index} ${field} value`, 128),
  ] as const);
  return Object.freeze(Object.fromEntries(entries));
}

function tokenTypeValue(value: unknown, owner: string): "string" | "other" | "comment" | "regex" {
  const tokenType = boundedText(value, owner, 32);
  if (tokenType !== "string" && tokenType !== "other" && tokenType !== "comment" && tokenType !== "regex") {
    throw new TypeError(`${owner} is invalid`);
  }
  return tokenType;
}

function selectorValue(value: string, owner: string): string {
  if (typeof value !== "string" || value.length === 0 || value.length > 512 || /[\r\n]/u.test(value) || value.trim() !== value) {
    throw new TypeError(`${owner} is invalid`);
  }
  return value;
}

function parseScopes(value: unknown, owner: string): readonly string[] {
  const scopes = parseTextList(value, owner);
  const normalized = scopes.map(scope => scopeNameValue(scope, owner));
  if (new Set(normalized).size !== normalized.length) throw new RangeError(`${owner} must be unique`);
  return Object.freeze(normalized);
}

function parseBracketScopes(value: unknown, owner: string): readonly string[] {
  const scopes = parseTextList(value, owner);
  const normalized = scopes.map(scope => {
    if (scope === "*") return scope;
    if (!/^[A-Za-z0-9][A-Za-z0-9._+*-]*$/u.test(scope)) throw new TypeError(`${owner} contains an invalid scope selector`);
    return scope;
  });
  if (new Set(normalized).size !== normalized.length) throw new RangeError(`${owner} must be unique`);
  return Object.freeze(normalized);
}

function parseTextList(value: unknown, owner: string, caseInsensitive = false, requireUnique = true): readonly string[] {
  if (value === undefined) return Object.freeze([]);
  if (!Array.isArray(value)) throw new TypeError(`${owner} must be an array`);
  const values = value.map(candidate => {
    const text = boundedText(candidate, owner, 256);
    return caseInsensitive ? text.toLowerCase() : text;
  });
  if (requireUnique && new Set(values).size !== values.length) throw new RangeError(`${owner} must be unique`);
  return Object.freeze(values);
}

function languageId(value: unknown, owner: string): string {
  const id = boundedText(value, owner, 128);
  if (!/^[A-Za-z0-9][A-Za-z0-9._-]*$/u.test(id)) throw new TypeError(`${owner} is invalid`);
  return id;
}

function scopeNameValue(value: unknown, owner: string): string {
  const scope = boundedText(value, owner, 256);
  if (!/^[A-Za-z0-9][A-Za-z0-9._+-]*$/u.test(scope)) throw new TypeError(`${owner} is invalid`);
  return scope;
}

function normalizeResourcePath(value: unknown, owner: string): string {
  let path = boundedText(value, owner, 1024);
  while (path.startsWith("./")) path = path.slice(2);
  if (path.length === 0 || path.includes("\\") || path.startsWith("/") || path.includes(":" ) || path.split("/").some(segment => segment.length === 0 || segment === "." || segment === "..")) {
    throw new TypeError(`${owner} must be a safe relative path`);
  }
  return path;
}

function record(value: unknown, owner: string): Record<string, unknown> {
  if (typeof value !== "object" || value === null || Array.isArray(value)) throw new TypeError(`${owner} must be an object`);
  return value as Record<string, unknown>;
}

function requiredString(value: unknown, owner: string, maximum: number): string {
  return boundedText(value, owner, maximum);
}

function boundedText(value: unknown, owner: string, maximum = 256): string {
  if (typeof value !== "string" || value.trim().length === 0 || value.length > maximum || /[\r\n]/u.test(value)) throw new TypeError(`${owner} is invalid`);
  return value;
}
