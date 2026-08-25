import { isPositiveSafeInteger } from "../../../../base/common/numbers.js";
import { VersionedLanguageResultStore } from "../languageResultStore.js";
import { parseLanguageCompletionSnippet } from "../../../contrib/snippet/common/snippetParser.js";
import { normalizeTextLineEndings, TextPosition, TextRange, type TextSnapshot } from "../../core/text.js";
import { type TextModel } from "../../model/textModel.js";

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

/** Selects whether completion insertion text is literal text or snippet syntax. */
export enum LanguageCompletionInsertTextFormat {
	PlainText = "plainText",
	Snippet = "snippet",
}

export interface LanguageCompletionItem {
	readonly providerId: string;
	readonly id: string;
	readonly label: string;
	readonly kind: LanguageCompletionItemKind;
	readonly range: TextRange;
	readonly insertText: string;
	readonly insertTextFormat?: LanguageCompletionInsertTextFormat;
	readonly detail?: string;
	readonly documentation?: string;
	readonly filterText?: string;
	readonly sortText?: string;
	readonly preselect?: boolean;
	/** Characters that atomically accept this item before being inserted. */
	readonly commitCharacters?: readonly string[];
	/** Additional non-overlapping edits applied with the primary completion replacement. */
	readonly additionalTextEdits?: readonly LanguageCompletionTextEdit[];
	/** Server command executed after this candidate has been inserted. */
	readonly command?: LanguageCompletionCommand;
	readonly hasDeferredDetails?: boolean;
}

/** One completion-owned server command. */
export interface LanguageCompletionCommand {
	readonly id: string;
	readonly title: string;
	readonly arguments: readonly unknown[];
}

/** One extra document replacement attached to a completion item. */
export interface LanguageCompletionTextEdit {
	readonly range: TextRange;
	readonly text: string;
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
		range => assertModelTextEditRange(model, range),
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
		range => assertSnapshotTextEditRange(lines, range),
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
	validateAdditionalRange: (range: TextRange) => void,
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
		if (item.insertTextFormat !== undefined && !Object.values(LanguageCompletionInsertTextFormat).includes(item.insertTextFormat)) {
			throw new TypeError(`Unknown language completion insert text format '${item.insertTextFormat}'`);
		}
		if (item.insertTextFormat === LanguageCompletionInsertTextFormat.Snippet) {
			parseLanguageCompletionSnippet(item.insertText, {
				allowUnresolvedVariables: true,
			});
		}
		assertOptionalText(item.detail, "Language completion item detail");
		assertOptionalText(item.documentation, "Language completion item documentation");
		assertOptionalText(item.filterText, "Language completion item filterText");
		assertOptionalText(item.sortText, "Language completion item sortText");
		if (item.preselect !== undefined && typeof item.preselect !== "boolean") {
			throw new TypeError("Language completion item preselect must be a boolean");
		}
		const commitCharacters = normalizeCommitCharacters(item.commitCharacters);
		const additionalTextEdits = normalizeAdditionalTextEdits(
			item.additionalTextEdits,
			item.range,
			validateAdditionalRange,
		);
		const command = normalizeCompletionCommand(item.command);
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
			...(item.insertTextFormat === undefined ? {} : { insertTextFormat: item.insertTextFormat }),
			...(item.detail === undefined ? {} : { detail: item.detail }),
			...(item.documentation === undefined ? {} : { documentation: item.documentation }),
			...(item.filterText === undefined ? {} : { filterText: item.filterText }),
			...(item.sortText === undefined ? {} : { sortText: item.sortText }),
			...(item.preselect === undefined ? {} : { preselect: item.preselect }),
			...(commitCharacters === undefined ? {} : { commitCharacters }),
			...(additionalTextEdits === undefined ? {} : { additionalTextEdits }),
			...(command === undefined ? {} : { command }),
			...(item.hasDeferredDetails === undefined ? {} : { hasDeferredDetails: item.hasDeferredDetails }),
		});
	});
	return Object.freeze({
		position: value.position,
		items: Object.freeze(items),
		isIncomplete: value.isIncomplete,
	});
}

function normalizeCompletionCommand(value: unknown): LanguageCompletionCommand | undefined {
	if (value === undefined) return undefined;
	if (typeof value !== "object" || value === null || Array.isArray(value)) throw new TypeError("Language completion command must be an object");
	const command = value as Record<string, unknown>;
	if (Object.keys(command).some(key => key !== "id" && key !== "title" && key !== "arguments")) throw new TypeError("Language completion command contains unsupported fields");
	assertIdentifier(command.id, "Language completion command ID");
	assertNonEmptyText(command.title, "Language completion command title");
	if (!Array.isArray(command.arguments)) throw new TypeError("Language completion command arguments must be an array");
	return Object.freeze({ id: command.id, title: command.title, arguments: Object.freeze(structuredClone(command.arguments)) });
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

function assertModelTextEditRange(model: TextModel, range: TextRange): void {
	if (!(range instanceof TextRange)) {
		throw new TypeError("Language completion additional edit range must be a TextRange");
	}
	model.offsetAt(range.start);
	model.offsetAt(range.end);
}

function assertSnapshotTextEditRange(lines: readonly string[], range: TextRange): void {
	if (!(range instanceof TextRange)) {
		throw new TypeError("Language completion additional edit range must be a TextRange");
	}
	assertSnapshotPosition(lines, range.start);
	assertSnapshotPosition(lines, range.end);
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

/** Validates one completion commit character using the same grapheme contract as browser input. */
export function assertLanguageCompletionCommitCharacter(value: unknown): asserts value is string {
	if (typeof value !== "string" || value === "\n" || value === "\r" || [...value].length !== 1) {
		throw new TypeError("Language completion commit character must be one non-line-break grapheme");
	}
}

function normalizeCommitCharacters(value: unknown): readonly string[] | undefined {
	if (value === undefined) return undefined;
	if (!Array.isArray(value)) throw new TypeError("Language completion commit characters must be an array");
	const characters = value.map(character => {
		assertLanguageCompletionCommitCharacter(character);
		return character;
	});
	if (new Set(characters).size !== characters.length) {
		throw new RangeError("Language completion commit characters must be unique");
	}
	return Object.freeze(characters);
}

function normalizeAdditionalTextEdits(value: unknown, primaryRange: TextRange, validateRange: (range: TextRange) => void): readonly LanguageCompletionTextEdit[] | undefined {
	if (value === undefined) return undefined;
	if (!Array.isArray(value)) throw new TypeError("Language completion additional text edits must be an array");
	const edits = value.map(edit => {
		if (typeof edit !== "object" || edit === null || Array.isArray(edit)) {
			throw new TypeError("Language completion additional text edit must be an object");
		}
		const record = edit as Record<string, unknown>;
		if (Object.keys(record).some(key => key !== "range" && key !== "text")) {
			throw new TypeError("Language completion additional text edit contains unsupported fields");
		}
		if (!(record.range instanceof TextRange)) {
			throw new TypeError("Language completion additional edit range must be a TextRange");
		}
		validateRange(record.range);
		if (typeof record.text !== "string") {
			throw new TypeError("Language completion additional edit text must be a string");
		}
		return Object.freeze({ range: record.range, text: normalizeTextLineEndings(record.text) });
	});
	assertNonOverlappingCompletionEditRanges(primaryRange, edits);
	return Object.freeze(edits);
}

function assertNonOverlappingCompletionEditRanges(primaryRange: TextRange, edits: readonly LanguageCompletionTextEdit[]): void {
	const ranges = [primaryRange, ...edits.map(edit => edit.range)].sort((left, right) =>
		left.start.compareTo(right.start) || left.end.compareTo(right.end)
	);
	for (let index = 1; index < ranges.length; index += 1) {
		const previous = ranges[index - 1]!;
		const current = ranges[index]!;
		if (current.start.compareTo(previous.end) <= 0) {
			throw new RangeError("Language completion primary and additional edit ranges must not overlap or touch");
		}
	}
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
	if (!isPositiveSafeInteger(value)) {
		throw new RangeError(`${owner} must be a positive safe integer`);
	}
}
