import { VersionedLanguageResultStore } from "./languageResultStore.js";
import { normalizeTextLineEndings, TextPosition, TextRange, type TextSnapshot } from "./text.js";
import { type TextModel } from "./textModel.js";

export enum LanguageCompletionItemKind {
  Text = "text",
  Method = "method",
  Function = "function",
  Constructor = "constructor",
  Field = "field",
  Variable = "variable",
  Class = "class",
  Interface = "interface",
  Module = "module",
  Property = "property",
  Unit = "unit",
  Value = "value",
  Enum = "enum",
  Keyword = "keyword",
  Snippet = "snippet",
  File = "file",
  Folder = "folder",
  Reference = "reference",
  TypeParameter = "typeParameter",
}

export interface LanguageCompletionItem {
  readonly providerId: string;
  readonly id: string;
  readonly label: string;
  readonly kind: LanguageCompletionItemKind;
  readonly range: TextRange;
  readonly insertText: string;
  readonly detail?: string;
  readonly documentation?: string;
  readonly filterText?: string;
  readonly sortText?: string;
  readonly preselect?: boolean;
  readonly hasDeferredDetails?: boolean;
}

export interface LanguageCompletionResult {
  readonly position: TextPosition;
  readonly items: readonly LanguageCompletionItem[];
  readonly isIncomplete: boolean;
}

export interface LanguageCompletionItemDetails {
  readonly detail?: string;
  readonly documentation?: string;
}

export interface LanguageCompletionResolveRequest {
  readonly completionRequestId: number;
  readonly modelVersion: number;
  readonly providerId: string;
  readonly itemId: string;
}

export interface LanguageCompletionItemResolver {
  resolveCompletionItem(request: LanguageCompletionResolveRequest, signal: AbortSignal): Promise<LanguageCompletionItemDetails>;
}

export type LanguageCompletionResultNormalizer = (value: LanguageCompletionResult) => LanguageCompletionResult;

export function createLanguageCompletionStore(model: TextModel): VersionedLanguageResultStore<LanguageCompletionResult> {
  return new VersionedLanguageResultStore(model, (value) => normalizeLanguageCompletionResult(
    value,
    position => model.offsetAt(position),
    (position, range) => assertModelCompletionRange(model, position, range),
  ));
}

/** Validates provider output against an immutable captured model snapshot. */
export function normalizeLanguageCompletionSnapshotResult(value: LanguageCompletionResult, snapshot: TextSnapshot): LanguageCompletionResult {
  return createLanguageCompletionSnapshotNormalizer(snapshot)(value);
}

/** Builds one reusable normalizer so all providers in a request share the same snapshot line index. */
export function createLanguageCompletionSnapshotNormalizer(snapshot: TextSnapshot): LanguageCompletionResultNormalizer {
  const lines = snapshot.getText().split("\n");
  return value => normalizeLanguageCompletionResult(
    value,
    position => assertSnapshotPosition(lines, position),
    (position, range) => assertSnapshotCompletionRange(lines, position, range),
  );
}

export function normalizeLanguageCompletionItemDetails(value: unknown): LanguageCompletionItemDetails {
  if (value === undefined) return EMPTY_DETAILS;
  if (typeof value !== "object" || value === null || Array.isArray(value)) {
    throw new TypeError("Language completion item details must be an object");
  }
  const details = value as Record<string, unknown>;
  for (const key of Object.keys(details)) {
    if (key !== "detail" && key !== "documentation") {
      throw new TypeError(`Language completion item details contain unsupported field '${key}'`);
    }
  }
  assertOptionalText(details.detail as string | undefined, "Language completion item detail");
  assertOptionalText(details.documentation as string | undefined, "Language completion item documentation");
  return Object.freeze({
    ...(details.detail === undefined ? {} : { detail: details.detail as string }),
    ...(details.documentation === undefined ? {} : { documentation: details.documentation as string }),
  });
}

export function normalizeLanguageCompletionResolveRequest(value: LanguageCompletionResolveRequest): LanguageCompletionResolveRequest {
  if (typeof value !== "object" || value === null) {
    throw new TypeError("Language completion resolve request must be an object");
  }
  assertPositiveSafeInteger(value.completionRequestId, "Language completion request ID");
  assertPositiveSafeInteger(value.modelVersion, "Language completion model version");
  assertIdentifier(value.providerId, "Language completion provider ID");
  assertIdentifier(value.itemId, "Language completion item ID");
  return Object.freeze({
    completionRequestId: value.completionRequestId,
    modelVersion: value.modelVersion,
    providerId: value.providerId,
    itemId: value.itemId,
  });
}

function normalizeLanguageCompletionResult(
  value: LanguageCompletionResult,
  validatePosition: (position: TextPosition) => void,
  validateRange: (position: TextPosition, range: TextRange) => void,
): LanguageCompletionResult {
  if (typeof value !== "object" || value === null || !Array.isArray(value.items)) {
    throw new TypeError("Language completion result must contain an items array");
  }
  if (!(value.position instanceof TextPosition)) {
    throw new TypeError("Language completion position must be a TextPosition");
  }
  validatePosition(value.position);
  if (typeof value.isIncomplete !== "boolean") {
    throw new TypeError("Language completion isIncomplete must be a boolean");
  }
  const identities = new Set<string>();
  let preselectSeen = false;
  const items = value.items.map(item => {
    if (typeof item !== "object" || item === null) {
      throw new TypeError("Language completion item must be an object");
    }
    assertIdentifier(item.providerId, "Language completion provider ID");
    assertIdentifier(item.id, "Language completion item ID");
    const identity = `${item.providerId}\0${item.id}`;
    if (identities.has(identity)) {
      throw new RangeError(`Duplicate language completion item identity '${item.providerId}/${item.id}'`);
    }
    identities.add(identity);
    assertNonEmptyText(item.label, "Language completion item label");
    if (!Object.values(LanguageCompletionItemKind).includes(item.kind)) {
      throw new TypeError(`Unknown language completion item kind '${item.kind}'`);
    }
    validateRange(value.position, item.range);
    if (typeof item.insertText !== "string") {
      throw new TypeError("Language completion insertText must be a string");
    }
    assertOptionalText(item.detail, "Language completion item detail");
    assertOptionalText(item.documentation, "Language completion item documentation");
    assertOptionalText(item.filterText, "Language completion item filterText");
    assertOptionalText(item.sortText, "Language completion item sortText");
    if (item.preselect !== undefined && typeof item.preselect !== "boolean") {
      throw new TypeError("Language completion item preselect must be a boolean");
    }
    if (item.hasDeferredDetails !== undefined && typeof item.hasDeferredDetails !== "boolean") {
      throw new TypeError("Language completion item hasDeferredDetails must be a boolean");
    }
    if (item.preselect) {
      if (preselectSeen) {
        throw new RangeError("Language completion result must not preselect multiple items");
      }
      preselectSeen = true;
    }
    return Object.freeze({
      providerId: item.providerId,
      id: item.id,
      label: item.label,
      kind: item.kind,
      range: item.range,
      insertText: normalizeTextLineEndings(item.insertText),
      ...(item.detail === undefined ? {} : { detail: item.detail }),
      ...(item.documentation === undefined ? {} : { documentation: item.documentation }),
      ...(item.filterText === undefined ? {} : { filterText: item.filterText }),
      ...(item.sortText === undefined ? {} : { sortText: item.sortText }),
      ...(item.preselect === undefined ? {} : { preselect: item.preselect }),
      ...(item.hasDeferredDetails === undefined ? {} : { hasDeferredDetails: item.hasDeferredDetails }),
    });
  });
  return Object.freeze({
    position: value.position,
    items: Object.freeze(items),
    isIncomplete: value.isIncomplete,
  });
}

const EMPTY_DETAILS: LanguageCompletionItemDetails = Object.freeze({});

function assertModelCompletionRange(model: TextModel, position: TextPosition, range: TextRange): void {
  if (!(range instanceof TextRange)) {
    throw new TypeError("Language completion item range must be a TextRange");
  }
  model.offsetAt(range.start);
  model.offsetAt(range.end);
  if (
    range.start.lineIndex !== position.lineIndex ||
    range.end.lineIndex !== position.lineIndex
  ) {
    throw new RangeError("Language completion item range must stay on the trigger line");
  }
  if (range.start.compareTo(position) > 0 || range.end.compareTo(position) < 0) {
    throw new RangeError("Language completion item range must contain the trigger position");
  }
}

function assertSnapshotCompletionRange(lines: readonly string[], position: TextPosition, range: TextRange): void {
  if (!(range instanceof TextRange)) {
    throw new TypeError("Language completion item range must be a TextRange");
  }
  assertSnapshotPosition(lines, range.start);
  assertSnapshotPosition(lines, range.end);
  if (
    range.start.lineIndex !== position.lineIndex ||
    range.end.lineIndex !== position.lineIndex
  ) {
    throw new RangeError("Language completion item range must stay on the trigger line");
  }
  if (range.start.compareTo(position) > 0 || range.end.compareTo(position) < 0) {
    throw new RangeError("Language completion item range must contain the trigger position");
  }
}

function assertSnapshotPosition(lines: readonly string[], position: TextPosition): void {
  if (!(position instanceof TextPosition)) {
    throw new TypeError("Language completion position must be a TextPosition");
  }
  if (
    position.lineIndex >= lines.length ||
    position.columnIndex > lines[position.lineIndex]!.length
  ) {
    throw new RangeError("Language completion position is outside its snapshot");
  }
}

function assertOptionalText(value: string | undefined, owner: string): void {
  if (value !== undefined) assertNonEmptyText(value, owner);
}

function assertIdentifier(value: unknown, owner: string): asserts value is string {
  assertNonEmptyText(value, owner);
  if (value.trim() !== value) {
    throw new TypeError(`${owner} must be trimmed`);
  }
}

function assertNonEmptyText(value: unknown, owner: string): asserts value is string {
  if (typeof value !== "string" || value.trim().length === 0) {
    throw new TypeError(`${owner} must not be empty`);
  }
}

function assertPositiveSafeInteger(value: unknown, owner: string): asserts value is number {
  if (!Number.isSafeInteger(value) || (value as number) <= 0) {
    throw new RangeError(`${owner} must be a positive safe integer`);
  }
}
