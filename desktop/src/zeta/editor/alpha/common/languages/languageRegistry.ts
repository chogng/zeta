import { Emitter, type Event } from "../../../../base/common/event.js";
import { DisposableOwner, toDisposable, type IDisposable } from "../../../../base/common/lifecycle.js";
import { assertLanguageId } from "./languageId.js";
import type { TextResourceLanguageInput } from "../../../common/textResourceLanguage.js";

/** Declarative identity and file-association metadata for one editor language. */
export interface LanguageDescription {
  readonly id: string;
  readonly aliases?: readonly string[];
  readonly extensions?: readonly string[];
  readonly filenames?: readonly string[];
  readonly filenamePatterns?: readonly string[];
  readonly mimetypes?: readonly string[];
}

export interface LanguageRegistrationOptions {
  readonly priority?: number;
}

export interface LanguageDescriptionChangeEvent {
  readonly languageId: string;
}

interface RegisteredLanguageDescription {
  readonly description: LanguageDescription;
  readonly priority: number;
  readonly order: number;
}

/** Resolves declarative language associations without owning extension resources. */
export class LanguageRegistry extends DisposableOwner {
  private readonly changeEmitter = this.own(new Emitter<LanguageDescriptionChangeEvent>());
  private readonly descriptions = new Map<string, RegisteredLanguageDescription[]>();
  private nextOrder = 1;
  private disposed = false;

  readonly onDidChange: Event<LanguageDescriptionChangeEvent> = this.changeEmitter.event;

  constructor() {
    super();
    this.defer(() => {
      this.disposed = true;
      this.descriptions.clear();
    });
  }

  register(description: LanguageDescription, options: LanguageRegistrationOptions = {}): IDisposable {
    this.ensureAlive();
    const normalized = normalizeDescription(description);
    const priority = normalizePriority(options);
    const entry = Object.freeze({ description: normalized, priority, order: this.nextOrder++ });
    const values = this.descriptions.get(normalized.id);
    if (values) values.push(entry);
    else this.descriptions.set(normalized.id, [entry]);
    this.changeEmitter.fire(Object.freeze({ languageId: normalized.id }));
    return toDisposable(() => {
      const current = this.descriptions.get(normalized.id);
      if (!current) return;
      const index = current.indexOf(entry);
      if (index < 0) return;
      current.splice(index, 1);
      if (current.length === 0) this.descriptions.delete(normalized.id);
      if (!this.disposed) this.changeEmitter.fire(Object.freeze({ languageId: normalized.id }));
    });
  }

  get(languageId: string): LanguageDescription | undefined {
    this.ensureAlive();
    assertLanguageId(languageId);
    return selectDescription(this.descriptions.get(languageId))?.description;
  }

  resolveLanguageId(input: TextResourceLanguageInput): string | undefined {
    this.ensureAlive();
    if (!input || typeof input !== "object") throw new TypeError("Language resolution input is required");
    const path = normalizePath(input.resource.path);
    const fileName = path.slice(path.lastIndexOf("/") + 1);
    const contentType = input.contentType?.toLowerCase();
    let best: LanguageMatch | undefined;
    for (const entries of this.descriptions.values()) {
      for (const entry of entries) {
        const match = matchDescription(entry, path, fileName, contentType);
        if (match && (!best || compareMatches(match, best) > 0)) best = match;
      }
    }
    return best?.languageId;
  }

  private ensureAlive(): void {
    if (this.disposed) throw new ReferenceError("LanguageRegistry is already disposed");
  }
}

interface LanguageMatch {
  readonly languageId: string;
  readonly priority: number;
  readonly rank: number;
  readonly specificity: number;
  readonly order: number;
}

function normalizeDescription(description: LanguageDescription): LanguageDescription {
  if (typeof description !== "object" || description === null) throw new TypeError("Language description must be an object");
  assertLanguageId(description.id);
  return Object.freeze({
    id: description.id,
    aliases: normalizeTextList(description.aliases, "language aliases"),
    extensions: normalizeExtensions(description.extensions),
    filenames: normalizeTextList(description.filenames, "language filenames", true),
    filenamePatterns: normalizeTextList(description.filenamePatterns, "language filename patterns", true),
    mimetypes: normalizeTextList(description.mimetypes, "language MIME types", true),
  });
}

function normalizePriority(options: LanguageRegistrationOptions): number {
  if (typeof options !== "object" || options === null) throw new TypeError("Language registration options must be an object");
  const priority = options.priority ?? 0;
  if (!Number.isSafeInteger(priority)) throw new RangeError("Language registration priority must be a safe integer");
  return priority;
}

function normalizeExtensions(values: readonly string[] | undefined): readonly string[] {
  const normalized = normalizeTextList(values, "language extensions", true).map(value => value.toLowerCase());
  if (normalized.some(value => !value.startsWith("."))) throw new TypeError("Language extensions must start with a dot");
  return Object.freeze(normalized);
}

function normalizeTextList(values: readonly string[] | undefined, owner: string, caseInsensitive = false): readonly string[] {
  if (values === undefined) return Object.freeze([]);
  if (!Array.isArray(values)) throw new TypeError(`${owner} must be an array`);
  const normalized = values.map(value => {
    if (typeof value !== "string" || value.length === 0 || value.length > 256 || /[\r\n]/u.test(value)) {
      throw new TypeError(`${owner} must contain bounded single-line strings`);
    }
    return caseInsensitive ? value.toLowerCase() : value;
  });
  if (new Set(normalized).size !== normalized.length) throw new RangeError(`${owner} must be unique`);
  return Object.freeze(normalized);
}

function normalizePath(path: string): string {
  return path.replace(/\\/gu, "/").toLowerCase();
}

function matchDescription(entry: RegisteredLanguageDescription, path: string, fileName: string, contentType: string | undefined): LanguageMatch | undefined {
  const description = entry.description;
  if (contentType && description.mimetypes?.includes(contentType)) {
    return match(entry, 4, contentType.length);
  }
  if (description.filenames?.includes(fileName)) {
    return match(entry, 3, fileName.length);
  }
  const pattern = description.filenamePatterns?.find(value => matchesGlob(value, path) || matchesGlob(value, fileName));
  if (pattern) return match(entry, 2, pattern.length);
  const extension = description.extensions?.filter(value => path.endsWith(value)).sort((left, right) => right.length - left.length)[0];
  if (extension) return match(entry, 1, extension.length);
  return undefined;
}

function match(entry: RegisteredLanguageDescription, rank: number, specificity: number): LanguageMatch {
  return { languageId: entry.description.id, priority: entry.priority, rank, specificity, order: entry.order };
}

function compareMatches(left: LanguageMatch, right: LanguageMatch): number {
  return left.priority - right.priority || left.rank - right.rank || left.specificity - right.specificity || left.order - right.order;
}

function selectDescription(entries: readonly RegisteredLanguageDescription[] | undefined): RegisteredLanguageDescription | undefined {
  return entries?.reduce((best, entry) => !best || entry.priority > best.priority || (entry.priority === best.priority && entry.order > best.order) ? entry : best, undefined as RegisteredLanguageDescription | undefined);
}

function matchesGlob(pattern: string, value: string): boolean {
  let expression = "^";
  for (let index = 0; index < pattern.length; index += 1) {
    const character = pattern[index]!;
    if (character === "*") {
      if (pattern[index + 1] === "*") {
        index += 1;
        expression += ".*";
      } else {
        expression += "[^/]*";
      }
    } else if (character === "?") {
      expression += "[^/]";
    } else {
      expression += escapeRegularExpression(character);
    }
  }
  try {
    return new RegExp(`${expression}$`, "u").test(value);
  } catch {
    return false;
  }
}

function escapeRegularExpression(value: string): string {
  return value.replace(/[|\\{}()[\]^$+?.]/gu, "\\$&");
}
