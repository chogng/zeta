import { SYNTAX_DIAGNOSTIC_LANE, SYNTAX_TOKEN_LANE, type SyntaxLane, type SyntaxResult } from "./syntaxService.js";
import { createSyntaxItemSplices, type SyntaxItem } from "./syntaxItemDelta.js";
import { attachLanguageTokenResultDelta, createLanguageDiagnosticSnapshotNormalizer, createLanguageTokenSnapshotNormalizer, type LanguageDiagnostic, type LanguageDiagnosticCode, type LanguageDiagnosticResult, type LanguageToken, type LanguageTokenResult } from "../languageResults.js";
import { type LanguageWorkerWireResultState } from "../languageWorkerWireProtocol.js";
import { Position } from "../../core/position.js";
import { Range } from "../../core/range.js";
import { type TextSnapshot } from "../../core/textChange.js";

export function encodeSyntaxWireResult(lane: SyntaxLane, result: SyntaxResult, snapshot: TextSnapshot, base: LanguageWorkerWireResultState<SyntaxResult> | undefined): unknown {
	assertResultLane(lane, result);
	const items = lane === SYNTAX_TOKEN_LANE
		? (result.value as LanguageTokenResult).tokens
		: (result.value as LanguageDiagnosticResult).diagnostics;
	const baseItems = readBaseItems(lane, base);
	if (!base || !baseItems) return encodeFull(lane, items);
	const splices = createSyntaxItemSplices(lane, baseItems, items, base.snapshot, snapshot);
	const insertedItemCount = splices.reduce((count, splice) => count + splice.items.length, 0);
	if (insertedItemCount >= items.length) return encodeFull(lane, items);
	return Object.freeze({
		kind: "delta",
		baseRequestId: base.requestId,
		splices: Object.freeze(splices.map(splice => Object.freeze({
			startItemIndex: splice.startItemIndex,
			deleteItemCount: splice.deleteItemCount,
			lineDelta: splice.lineDeltaAfter,
			items: Object.freeze(splice.items.map(item => encodeItem(lane, item))),
		}))),
	});
}

export function decodeSyntaxWireResult(lane: SyntaxLane, value: unknown, snapshot: TextSnapshot, base: LanguageWorkerWireResultState<SyntaxResult> | undefined): SyntaxResult {
	assertRecord(value, "Syntax wire result");
	if (value.kind === "full") {
		if (!Array.isArray(value.items)) throw new TypeError("Full syntax wire result must contain items");
		return resultFromItems(lane, value.items.map(item => decodeItem(lane, item)), snapshot);
	}
	if (value.kind !== "delta") {
		throw new TypeError(`Unknown syntax wire result kind '${String(value.kind)}'`);
	}
	if (!Array.isArray(value.splices)) {
		throw new TypeError("Syntax delta must contain splices");
	}
	const baseRequestId = decodePositiveSafeInteger(value.baseRequestId, "Syntax delta base request ID");
	if (!base || base.requestId !== baseRequestId) {
		throw new Error("Syntax delta base result is unavailable");
	}
	const baseItems = readBaseItems(lane, base);
	if (!baseItems) {
		throw new Error("Syntax delta base lane does not match");
	}
	const items: SyntaxItem[] = [];
	const tokenSplices = [];
	let baseItemIndex = 0;
	let lineDelta = 0;
	for (const encodedSplice of value.splices) {
		assertRecord(encodedSplice, "Syntax delta splice");
		if (!Array.isArray(encodedSplice.items)) {
			throw new TypeError("Syntax delta splice must contain items");
		}
		const startItemIndex = decodeNonNegativeSafeInteger(encodedSplice.startItemIndex, "Syntax delta start item index");
		const deleteItemCount = decodeNonNegativeSafeInteger(encodedSplice.deleteItemCount, "Syntax delta delete item count");
		if (startItemIndex < baseItemIndex || startItemIndex > baseItems.length || deleteItemCount > baseItems.length - startItemIndex) {
			throw new RangeError("Syntax delta splices must be ordered, non-overlapping, and inside their base result");
		}
		for (const item of baseItems.slice(baseItemIndex, startItemIndex)) items.push(shiftItem(lane, item, lineDelta));
		const inserted = encodedSplice.items.map(item => decodeItem(lane, item));
		const resultStartItemIndex = items.length;
		items.push(...inserted);
		const nextLineDelta = decodeSafeInteger(encodedSplice.lineDelta, "Syntax delta line shift");
		tokenSplices.push(Object.freeze({
			baseStartItemIndex: startItemIndex,
			baseDeleteItemCount: deleteItemCount,
			resultStartItemIndex,
			resultInsertItemCount: inserted.length,
			lineDeltaBefore: lineDelta,
			lineDeltaAfter: nextLineDelta,
		}));
		baseItemIndex = startItemIndex + deleteItemCount;
		lineDelta = nextLineDelta;
	}
	if (lineDelta !== snapshot.lineCount - base.snapshot.lineCount) {
		throw new Error("Syntax delta final line shift does not match its snapshots");
	}
	for (const item of baseItems.slice(baseItemIndex)) items.push(shiftItem(lane, item, lineDelta));
	const result = resultFromItems(lane, items, snapshot);
	if (result.lane === SYNTAX_TOKEN_LANE) {
		attachLanguageTokenResultDelta(result.value, {
			baseRequestId,
			splices: tokenSplices,
		});
	}
	return result;
}

type SyntaxWireItem = SyntaxItem;

function encodeFull(lane: SyntaxLane, items: readonly SyntaxWireItem[]): unknown {
	return Object.freeze({
		kind: "full",
		items: Object.freeze(items.map(item => encodeItem(lane, item))),
	});
}

function readBaseItems(lane: SyntaxLane, base: LanguageWorkerWireResultState<SyntaxResult> | undefined): readonly SyntaxWireItem[] | undefined {
	if (!base || base.result.lane !== lane) return undefined;
	return lane === SYNTAX_TOKEN_LANE
		? (base.result.value as LanguageTokenResult).tokens
		: (base.result.value as LanguageDiagnosticResult).diagnostics;
}

function resultFromItems(lane: SyntaxLane, items: readonly SyntaxWireItem[], snapshot: TextSnapshot): SyntaxResult {
	return lane === SYNTAX_TOKEN_LANE
		? Object.freeze({
			lane: SYNTAX_TOKEN_LANE,
			value: createLanguageTokenSnapshotNormalizer(snapshot)({ tokens: items as readonly LanguageToken[] }),
		})
		: Object.freeze({
			lane: SYNTAX_DIAGNOSTIC_LANE,
			value: createLanguageDiagnosticSnapshotNormalizer(snapshot)({ diagnostics: items as readonly LanguageDiagnostic[] }),
		});
}

function encodeItem(lane: SyntaxLane, item: SyntaxWireItem): unknown {
	if (lane === SYNTAX_TOKEN_LANE) {
		const token = item as LanguageToken;
		return Object.freeze({
			range: encodeRange(token.range, "Language token wire range"),
			tokenType: token.tokenType,
			modifiers: Object.freeze([...token.modifiers]),
			...(token.languageId === undefined ? {} : { languageId: token.languageId }),
			...(token.balancedBrackets === undefined ? {} : { balancedBrackets: token.balancedBrackets }),
			...(token.presentation === undefined ? {} : { presentation: token.presentation }),
		});
	}
	const diagnostic = item as LanguageDiagnostic;
	return Object.freeze({
		range: encodeRange(diagnostic.range, "Language diagnostic wire range"),
		severity: diagnostic.severity,
		message: diagnostic.message,
		...(diagnostic.code === undefined ? {} : { code: diagnostic.code }),
		...(diagnostic.source === undefined ? {} : { source: diagnostic.source }),
	});
}

function decodeItem(lane: SyntaxLane, value: unknown): SyntaxWireItem {
	assertRecord(value, lane === SYNTAX_TOKEN_LANE ? "Language token wire token" : "Language diagnostic wire diagnostic");
	if (lane === SYNTAX_TOKEN_LANE) {
		if (!Array.isArray(value.modifiers)) {
			throw new TypeError("Language token wire modifiers must be an array");
		}
		return {
			range: decodeRange(value.range, "Language token wire range"),
			tokenType: decodeString(value.tokenType, "Language token wire type"),
			modifiers: value.modifiers.map(modifier => decodeString(modifier, "Language token wire modifier")),
			...(value.languageId === undefined ? {} : { languageId: decodeString(value.languageId, "Language token wire embedded language ID") }),
			...(value.balancedBrackets === undefined ? {} : { balancedBrackets: decodeExcludedBrackets(value.balancedBrackets) }),
			...(value.presentation === undefined ? {} : { presentation: decodePresentation(value.presentation) }),
		};
	}
	const code = decodeDiagnosticCode(value.code);
	const source = value.source === undefined ? undefined : decodeString(value.source, "Language diagnostic wire source");
	return {
		range: decodeRange(value.range, "Language diagnostic wire range"),
		severity: decodeString(value.severity, "Language diagnostic wire severity") as LanguageDiagnostic["severity"],
		message: decodeString(value.message, "Language diagnostic wire message"),
		...(code === undefined ? {} : { code }),
		...(source === undefined ? {} : { source }),
	};
}

function decodeExcludedBrackets(value: unknown): false {
	if (value !== false) throw new TypeError("Language token wire balanced-bracket metadata must be false");
	return false;
}

function decodePresentation(value: unknown): NonNullable<LanguageToken["presentation"]> {
	assertRecord(value, "Language token wire presentation");
	const foreground = value.foreground === undefined ? undefined : decodeString(value.foreground, "Language token wire foreground");
	const background = value.background === undefined ? undefined : decodeString(value.background, "Language token wire background");
	if (value.fontStyle !== undefined && !Array.isArray(value.fontStyle)) throw new TypeError("Language token wire font style must be an array");
	const fontStyle = value.fontStyle === undefined ? undefined : value.fontStyle.map(style => decodeString(style, "Language token wire font style")) as NonNullable<LanguageToken["presentation"]>["fontStyle"];
	return { ...(foreground === undefined ? {} : { foreground }), ...(background === undefined ? {} : { background }), ...(fontStyle === undefined ? {} : { fontStyle }) };
}

function shiftItem(lane: SyntaxLane, item: SyntaxWireItem, lineDelta: number): SyntaxWireItem {
	const range = Range.fromPositions(
		new Position(item.range.startLineNumber + lineDelta, item.range.startColumn),
		new Position(item.range.endLineNumber + lineDelta, item.range.endColumn),
	);
	return lane === SYNTAX_TOKEN_LANE
		? { ...(item as LanguageToken), range }
		: { ...(item as LanguageDiagnostic), range };
}

function encodeRange(range: Range, owner: string): unknown {
	if (!(range instanceof Range)) throw new TypeError(`${owner} must be a Range`);
	return Object.freeze({
		start: Object.freeze({ lineIndex: range.startLineNumber - 1, columnIndex: range.startColumn - 1 }),
		end: Object.freeze({ lineIndex: range.endLineNumber - 1, columnIndex: range.endColumn - 1 }),
	});
}

function decodeRange(value: unknown, owner: string): Range {
	assertRecord(value, owner);
	return Range.fromPositions(decodePosition(value.start, `${owner} start`), decodePosition(value.end, `${owner} end`));
}

function decodePosition(value: unknown, owner: string): Position {
	assertRecord(value, owner);
	return new Position((decodeNonNegativeSafeInteger(value.lineIndex, `${owner} line index`)) + 1, (decodeNonNegativeSafeInteger(value.columnIndex, `${owner} column index`)) + 1);
}

function decodeDiagnosticCode(value: unknown): LanguageDiagnosticCode | undefined {
	if (value === undefined || typeof value === "string") return value;
	if (typeof value === "number" && Number.isFinite(value)) return value;
	throw new TypeError("Language diagnostic wire code must be a finite number or string");
}

function assertResultLane(lane: SyntaxLane, result: SyntaxResult): void {
	if (!result || result.lane !== lane) throw new TypeError(`Syntax wire result does not match lane '${lane}'`);
}

function decodeString(value: unknown, owner: string): string {
	if (typeof value !== "string") throw new TypeError(`${owner} must be a string`);
	return value;
}

function decodePositiveSafeInteger(value: unknown, owner: string): number {
	const decoded = decodeNonNegativeSafeInteger(value, owner);
	if (decoded === 0) throw new RangeError(`${owner} must be positive`);
	return decoded;
}

function decodeNonNegativeSafeInteger(value: unknown, owner: string): number {
	if (!Number.isSafeInteger(value) || (value as number) < 0) throw new RangeError(`${owner} must be a non-negative safe integer`);
	return value as number;
}

function decodeSafeInteger(value: unknown, owner: string): number {
	if (!Number.isSafeInteger(value)) throw new RangeError(`${owner} must be a safe integer`);
	return value as number;
}

function assertRecord(value: unknown, owner: string): asserts value is Record<string, unknown> {
	if (typeof value !== "object" || value === null || Array.isArray(value)) throw new TypeError(`${owner} must be an object`);
}
