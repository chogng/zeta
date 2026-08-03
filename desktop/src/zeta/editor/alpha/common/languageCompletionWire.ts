import { LANGUAGE_COMPLETION_LANE, type LanguageCompletionLane } from "./languageCompletionService.js";
import { createLanguageCompletionIncompleteRefreshContext, createLanguageCompletionInvokeContext, createLanguageCompletionTriggerCharacterContext, LanguageCompletionTriggerKind, type LanguageCompletionContext, type LanguageCompletionRequest } from "./languageCompletionProviders.js";
import { normalizeLanguageCompletionSnapshotResult, type LanguageCompletionItem, type LanguageCompletionResult } from "./languageCompletions.js";
import { type LanguageWorkerWireCodec } from "./languageWorkerWire.js";
import { TextPosition, TextRange, type TextSnapshot } from "./text.js";

export const languageCompletionWireCodec: LanguageWorkerWireCodec<LanguageCompletionLane, LanguageCompletionRequest, LanguageCompletionResult> = Object.freeze({
  lanes: Object.freeze([LANGUAGE_COMPLETION_LANE] as const),
  resultProtocol: "stateless",
  encodePayload(_lane: LanguageCompletionLane, request: LanguageCompletionRequest): unknown {
    return encodeCompletionRequest(request);
  },
  decodePayload(_lane: LanguageCompletionLane, value: unknown, snapshot: TextSnapshot): LanguageCompletionRequest {
    return decodeCompletionRequest(value, snapshot);
  },
  encodeResult(_lane: LanguageCompletionLane, result: LanguageCompletionResult): unknown {
    return encodeCompletionResult(result);
  },
  decodeResult(_lane: LanguageCompletionLane, value: unknown, snapshot: TextSnapshot): LanguageCompletionResult {
    return decodeCompletionResult(value, snapshot);
  },
});

function encodeCompletionRequest(request: LanguageCompletionRequest): unknown {
  return Object.freeze({
    languageId: request.languageId,
    position: encodePosition(request.position),
    context: encodeContext(request.context),
  });
}

function decodeCompletionRequest(value: unknown, snapshot: TextSnapshot): LanguageCompletionRequest {
  assertRecord(value, "Completion wire request");
  if (typeof value.languageId !== "string") {
    throw new TypeError("Completion wire language ID must be a string");
  }
  const position = decodePosition(value.position, "Completion wire request position");
  assertSnapshotPosition(snapshot, position);
  return Object.freeze({
    languageId: value.languageId,
    position,
    context: decodeContext(value.context),
  });
}

function encodeCompletionResult(result: LanguageCompletionResult): unknown {
  return Object.freeze({
    position: encodePosition(result.position),
    items: Object.freeze(result.items.map(encodeItem)),
    isIncomplete: result.isIncomplete,
  });
}

function decodeCompletionResult(value: unknown, snapshot: TextSnapshot): LanguageCompletionResult {
  assertRecord(value, "Completion wire result");
  if (!Array.isArray(value.items)) {
    throw new TypeError("Completion wire result must contain an items array");
  }
  if (typeof value.isIncomplete !== "boolean") {
    throw new TypeError("Completion wire isIncomplete must be a boolean");
  }
  return normalizeLanguageCompletionSnapshotResult({
    position: decodePosition(value.position, "Completion wire result position"),
    items: value.items.map(decodeItem),
    isIncomplete: value.isIncomplete,
  }, snapshot);
}

function encodeItem(item: LanguageCompletionItem): unknown {
  return Object.freeze({
    providerId: item.providerId,
    id: item.id,
    label: item.label,
    kind: item.kind,
    range: Object.freeze({
      start: encodePosition(item.range.start),
      end: encodePosition(item.range.end),
    }),
    insertText: item.insertText,
    ...(item.insertTextFormat === undefined ? {} : { insertTextFormat: item.insertTextFormat }),
    ...(item.detail === undefined ? {} : { detail: item.detail }),
    ...(item.documentation === undefined ? {} : { documentation: item.documentation }),
    ...(item.filterText === undefined ? {} : { filterText: item.filterText }),
    ...(item.sortText === undefined ? {} : { sortText: item.sortText }),
    ...(item.preselect === undefined ? {} : { preselect: item.preselect }),
    ...(item.commitCharacters === undefined ? {} : { commitCharacters: item.commitCharacters }),
    ...(item.additionalTextEdits === undefined ? {} : { additionalTextEdits: item.additionalTextEdits.map(edit => Object.freeze({
      range: Object.freeze({ start: encodePosition(edit.range.start), end: encodePosition(edit.range.end) }),
      text: edit.text,
    })) }),
    ...(item.hasDeferredDetails === undefined ? {} : { hasDeferredDetails: item.hasDeferredDetails }),
  });
}

function decodeItem(value: unknown): LanguageCompletionItem {
  assertRecord(value, "Completion wire item");
  const range = value.range;
  assertRecord(range, "Completion wire item range");
  const detail = decodeOptionalString(value.detail, "Completion wire item detail");
  const documentation = decodeOptionalString(value.documentation, "Completion wire item documentation");
  const filterText = decodeOptionalString(value.filterText, "Completion wire item filter text");
  const sortText = decodeOptionalString(value.sortText, "Completion wire item sort text");
  const commitCharacters = decodeOptionalCommitCharacters(value.commitCharacters);
  const additionalTextEdits = decodeOptionalAdditionalTextEdits(value.additionalTextEdits);
  if (value.preselect !== undefined && typeof value.preselect !== "boolean") {
    throw new TypeError("Completion wire item preselect must be a boolean");
  }
  if (value.hasDeferredDetails !== undefined && typeof value.hasDeferredDetails !== "boolean") {
    throw new TypeError("Completion wire item hasDeferredDetails must be a boolean");
  }
  return {
    providerId: decodeString(value.providerId, "Completion wire provider ID"),
    id: decodeString(value.id, "Completion wire item ID"),
    label: decodeString(value.label, "Completion wire item label"),
    kind: decodeString(value.kind, "Completion wire item kind") as LanguageCompletionItem["kind"],
    range: TextRange.from(
      decodePosition(range.start, "Completion wire item range start"),
      decodePosition(range.end, "Completion wire item range end"),
    ),
    insertText: decodeString(value.insertText, "Completion wire item insertion text"),
    ...(value.insertTextFormat === undefined ? {} : { insertTextFormat: decodeString(value.insertTextFormat, "Completion wire item insert text format") as LanguageCompletionItem["insertTextFormat"] }),
    ...(detail === undefined ? {} : { detail }),
    ...(documentation === undefined ? {} : { documentation }),
    ...(filterText === undefined ? {} : { filterText }),
    ...(sortText === undefined ? {} : { sortText }),
    ...(value.preselect === undefined ? {} : { preselect: value.preselect }),
    ...(commitCharacters === undefined ? {} : { commitCharacters }),
    ...(additionalTextEdits === undefined ? {} : { additionalTextEdits }),
    ...(value.hasDeferredDetails === undefined ? {} : { hasDeferredDetails: value.hasDeferredDetails }),
  };
}

function decodeOptionalCommitCharacters(value: unknown): readonly string[] | undefined {
  if (value === undefined) return undefined;
  if (!Array.isArray(value)) throw new TypeError("Completion wire item commit characters must be an array");
  return value.map(character => decodeString(character, "Completion wire item commit character"));
}

function decodeOptionalAdditionalTextEdits(value: unknown): readonly { readonly range: TextRange; readonly text: string }[] | undefined {
  if (value === undefined) return undefined;
  if (!Array.isArray(value)) throw new TypeError("Completion wire item additional text edits must be an array");
  return value.map(edit => {
    assertRecord(edit, "Completion wire additional text edit");
    assertRecord(edit.range, "Completion wire additional text edit range");
    return {
      range: TextRange.from(
        decodePosition(edit.range.start, "Completion wire additional text edit range start"),
        decodePosition(edit.range.end, "Completion wire additional text edit range end"),
      ),
      text: decodeString(edit.text, "Completion wire additional text edit text"),
    };
  });
}

function encodeContext(context: LanguageCompletionContext): unknown {
  return context.kind === LanguageCompletionTriggerKind.TriggerCharacter
    ? Object.freeze({ kind: context.kind, triggerCharacter: context.triggerCharacter })
    : Object.freeze({ kind: context.kind });
}

function decodeContext(value: unknown): LanguageCompletionContext {
  assertRecord(value, "Completion wire context");
  if (value.kind === LanguageCompletionTriggerKind.Invoke) {
    return createLanguageCompletionInvokeContext();
  }
  if (value.kind === LanguageCompletionTriggerKind.IncompleteRefresh) {
    return createLanguageCompletionIncompleteRefreshContext();
  }
  if (value.kind === LanguageCompletionTriggerKind.TriggerCharacter) {
    return createLanguageCompletionTriggerCharacterContext(
      decodeString(value.triggerCharacter, "Completion wire trigger character"),
    );
  }
  throw new TypeError(`Unknown completion wire trigger kind '${String(value.kind)}'`);
}

function encodePosition(position: TextPosition): unknown {
  if (!(position instanceof TextPosition)) {
    throw new TypeError("Completion wire position must be a TextPosition");
  }
  return Object.freeze({
    lineIndex: position.lineIndex,
    columnIndex: position.columnIndex,
  });
}

function decodePosition(value: unknown, owner: string): TextPosition {
  assertRecord(value, owner);
  return TextPosition.at(
    decodeNonNegativeSafeInteger(value.lineIndex, `${owner} line index`),
    decodeNonNegativeSafeInteger(value.columnIndex, `${owner} column index`),
  );
}

function assertSnapshotPosition(snapshot: TextSnapshot, position: TextPosition): void {
  const lines = snapshot.getText().split("\n");
  if (position.lineIndex >= lines.length || position.columnIndex > lines[position.lineIndex]!.length) {
    throw new RangeError("Completion wire request position is outside its snapshot");
  }
}

function decodeString(value: unknown, owner: string): string {
  if (typeof value !== "string") {
    throw new TypeError(`${owner} must be a string`);
  }
  return value;
}

function decodeOptionalString(value: unknown, owner: string): string | undefined {
  return value === undefined ? undefined : decodeString(value, owner);
}

function decodeNonNegativeSafeInteger(value: unknown, owner: string): number {
  if (!Number.isSafeInteger(value) || (value as number) < 0) {
    throw new RangeError(`${owner} must be a non-negative safe integer`);
  }
  return value as number;
}

function assertRecord(value: unknown, owner: string): asserts value is Record<string, unknown> {
  if (typeof value !== "object" || value === null || Array.isArray(value)) {
    throw new TypeError(`${owner} must be an object`);
  }
}
