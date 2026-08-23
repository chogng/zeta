import { DisposableOwner } from "../../../base/common/lifecycle.js";
import type { DiffApiHunk, DiffApiRange, DiffApiResult, DiffApiRow, IDiffApi } from "../../../platform/diff/common/diffApi.js";
import type { DiffComputationRequest, IDiffComputationService } from "../../common/diff/diffComputationService.js";
import { LineDiffKind, type DiffRange, type LineDiff, type LineDiffHunk, type LineDiffRow } from "../../common/diff/lineDiff.js";

/** Adapts the Rust diff projection to the editor's zero-based UTF-16 line model. */
export class RustDiffComputationService extends DisposableOwner implements IDiffComputationService {
	private disposed = false;

	constructor(private readonly api: IDiffApi) {
		super();
		if (!api || typeof api.compute !== "function") {
			this.dispose();
			throw new TypeError("Rust diff computation service requires a diff API");
		}
		this.defer(() => {
			this.disposed = true;
		});
	}

	async compute(request: DiffComputationRequest, signal: AbortSignal): Promise<LineDiff> {
		if (this.disposed) throw new ReferenceError("Rust diff computation service is already disposed");
		signal.throwIfAborted();
		const original = request.original.text;
		const modified = request.modified.text;
		const result = await this.api.compute({
			original: rustLineProjection(original),
			modified: rustLineProjection(modified),
		});
		signal.throwIfAborted();
		return projectResult(result, original, modified);
	}
}

function projectResult(result: DiffApiResult, originalText: string, modifiedText: string): LineDiff {
	const originalLines = originalText.split("\n");
	const modifiedLines = modifiedText.split("\n");
	if (result.originalLineCount !== originalLines.length || result.modifiedLineCount !== modifiedLines.length) {
		throw new Error("Rust diff result does not match the requested line model");
	}
	const rows = Object.freeze(result.rows.map(row => projectRow(row, originalLines, modifiedLines)));
	return Object.freeze({
		rows,
		hunks: Object.freeze(projectHunks(result.hunks, rows.length, originalLines.length, modifiedLines.length)),
	});
}

function projectRow(row: DiffApiRow, originalLines: readonly string[], modifiedLines: readonly string[]): LineDiffRow {
	const originalLineIndex = lineIndex(row.originalLineIndex, originalLines.length, "original");
	const modifiedLineIndex = lineIndex(row.modifiedLineIndex, modifiedLines.length, "modified");
	return Object.freeze({
		kind: rowKind(row.kind),
		...(originalLineIndex === undefined ? {} : { originalLineIndex }),
		...(modifiedLineIndex === undefined ? {} : { modifiedLineIndex }),
		originalChanges: Object.freeze(projectRanges(row.originalChanges, originalLineIndex === undefined ? undefined : originalLines[originalLineIndex], "original")),
		modifiedChanges: Object.freeze(projectRanges(row.modifiedChanges, modifiedLineIndex === undefined ? undefined : modifiedLines[modifiedLineIndex], "modified")),
	});
}

function rowKind(kind: DiffApiRow["kind"]): LineDiffKind {
	switch (kind) {
		case "context": return LineDiffKind.Unchanged;
		case "added": return LineDiffKind.Added;
		case "removed": return LineDiffKind.Removed;
		case "modified": return LineDiffKind.Modified;
	}
	throw new TypeError(`Unknown Rust diff row kind: ${String(kind)}`);
}

function lineIndex(lineIndex: number | null, lineCount: number, side: string): number | undefined {
	if (lineIndex === null) return undefined;
	if (!Number.isSafeInteger(lineIndex) || lineIndex < 0 || lineIndex >= lineCount) {
		throw new RangeError(`Rust ${side} diff line index is outside the requested text`);
	}
	return lineIndex;
}

function projectRanges(ranges: readonly DiffApiRange[], text: string | undefined, side: "original" | "modified"): readonly DiffRange[] {
	if (text === undefined && ranges.length > 0) throw new RangeError(`Rust ${side} diff ranges require a line`);
	let previousEnd = 0;
	return ranges.map(range => {
		if (!Number.isSafeInteger(range.startColumn) || !Number.isSafeInteger(range.endColumn) || range.startColumn < 0 || range.endColumn < range.startColumn || range.startColumn < previousEnd || (text !== undefined && range.endColumn > text.length)) {
			throw new RangeError(`Rust ${side} diff range is invalid`);
		}
		previousEnd = range.endColumn;
		return Object.freeze({ startColumn: range.startColumn, endColumn: range.endColumn });
	});
}

function projectHunks(hunks: readonly DiffApiHunk[], rowCount: number, originalLineCount: number, modifiedLineCount: number): readonly LineDiffHunk[] {
	let previousRowEnd = 0;
	return hunks.map(hunk => {
		if (!Number.isSafeInteger(hunk.rowStart) || !Number.isSafeInteger(hunk.rowEnd) || hunk.rowStart < previousRowEnd || hunk.rowEnd <= hunk.rowStart || hunk.rowEnd > rowCount) {
			throw new RangeError("Rust diff hunk row range is invalid");
		}
		if (!validLineSpan(hunk.originalStartLineIndex, hunk.originalLineCount, originalLineCount) || !validLineSpan(hunk.modifiedStartLineIndex, hunk.modifiedLineCount, modifiedLineCount)) {
			throw new RangeError("Rust diff hunk line range is invalid");
		}
		previousRowEnd = hunk.rowEnd;
		return Object.freeze({
			rowStart: hunk.rowStart,
			rowEnd: hunk.rowEnd,
			originalStartLineIndex: hunk.originalStartLineIndex,
			originalLineCount: hunk.originalLineCount,
			modifiedStartLineIndex: hunk.modifiedStartLineIndex,
			modifiedLineCount: hunk.modifiedLineCount,
		});
	});
}

function validLineSpan(start: number, count: number, lineCount: number): boolean {
	return Number.isSafeInteger(start) && Number.isSafeInteger(count) && start >= 0 && count >= 0 && start <= lineCount && count <= lineCount - start;
}

function rustLineProjection(text: string): string {
	return `${text}\n`;
}
