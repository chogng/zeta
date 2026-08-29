import { Disposable, toDisposable } from "../../../base/common/lifecycle.js";
import { type CursorsController } from "./cursor.js";
import { type LanguageCharacterPair } from "../languages/languageConfiguration.js";
import { type LanguageAutoClosingAction, type LanguageAutoClosingTrust } from "./languagePairEditing.js";
import { Position } from "../core/position.js";
import { Range } from "../core/range.js";
import { type TextModel } from "../model/textModel.js";
import { TrackedRangeStickiness, type TrackedRange } from "../model/trackedRange.js";

interface AutoClosingEntry {
	readonly open: string;
	readonly close: string;
	readonly enclosingRange: TrackedRange;
	readonly closeRange: TrackedRange;
}

/**
 * Tracks the auto-closed characters owned by one editor instance.
 *
 * The tracker borrows its model and selection controller. Callers record only
 * actions committed by that controller and dispose the tracker before either
 * borrowed dependency.
 */
export class LanguageAutoClosingTracker extends Disposable implements LanguageAutoClosingTrust {
	private entries: AutoClosingEntry[] = [];

	constructor(private readonly model: TextModel, private readonly selections: CursorsController) {
		super();
		if (model !== selections.textModel) {
			this.dispose();
			throw new TypeError("Language auto-closing dependencies must share one text model");
		}
		this._register(model.onDidChange(() => this.pruneInvalidEntries()));
		this._register(selections.onDidChange(() => this.pruneInvalidEntries()));
		this._register(toDisposable(() => {
			for (const entry of this.entries) this.disposeEntry(entry);
			this.entries = [];
		}));
	}

	record(actions: readonly LanguageAutoClosingAction[], committedModelVersion: number): void {
		this.assertNotDisposed();
		if (!Array.isArray(actions)) throw new TypeError("Auto-closing actions must be an array");
		if (!Number.isSafeInteger(committedModelVersion) || committedModelVersion < 1) {
			throw new RangeError("Committed model version must be a positive safe integer");
		}
		if (committedModelVersion !== this.model.version) return;
		for (const action of actions) assertAction(this.model, action);
		const additions: AutoClosingEntry[] = [];
		try {
			for (const action of actions) additions.push(this.createEntry(action));
		} catch (error) {
			for (const entry of additions) this.disposeEntry(entry);
			throw error;
		}
		this.entries.push(...additions);
		this.pruneInvalidEntries();
	}

	canOvertype(position: Position, close: string): boolean {
		this.assertNotDisposed();
		if (typeof close !== "string" || close.length === 0) return false;
		this.pruneInvalidEntries();
		return this.entries.some(entry => {
			if (entry.close !== close) return false;
			const range = entry.closeRange.range;
			return Position.compare(range.getStartPosition(), position) === 0;
		});
	}

	canDeletePair(position: Position, pair: LanguageCharacterPair): boolean {
		this.assertNotDisposed();
		this.pruneInvalidEntries();
		return this.entries.some(entry => {
			if (entry.open !== pair.open || entry.close !== pair.close) return false;
			const enclosing = entry.enclosingRange.range;
			const closer = entry.closeRange.range;
			if (Position.compare(closer.getStartPosition(), position) !== 0) return false;
			return this.model.offsetAt(closer.getStartPosition()) - this.model.offsetAt(enclosing.getStartPosition()) === entry.open.length;
		});
	}

	private createEntry(action: LanguageAutoClosingAction): AutoClosingEntry {
		const enclosingRange = this.model.trackRange(
			Range.fromPositions(this.model.positionAt(action.enclosingStartOffset), this.model.positionAt(action.closeEndOffset)),
			TrackedRangeStickiness.NeverGrowsAtEdges,
		);
		try {
			const closeRange = this.model.trackRange(
				Range.fromPositions(this.model.positionAt(action.closeStartOffset), this.model.positionAt(action.closeEndOffset)),
				TrackedRangeStickiness.NeverGrowsAtEdges,
			);
			return { open: action.open, close: action.close, enclosingRange, closeRange };
		} catch (error) {
			enclosingRange.dispose();
			throw error;
		}
	}

	private pruneInvalidEntries(): void {
		if (this.isDisposed) return;
		const retained: AutoClosingEntry[] = [];
		for (const entry of this.entries) {
			if (this.isEntryValid(entry)) retained.push(entry);
			else this.disposeEntry(entry);
		}
		this.entries = retained;
	}

	private isEntryValid(entry: AutoClosingEntry): boolean {
		const enclosing = entry.enclosingRange.range;
		const closer = entry.closeRange.range;
		if (enclosing.getStartPosition().lineNumber !== enclosing.getEndPosition().lineNumber || closer.getStartPosition().lineNumber !== closer.getEndPosition().lineNumber) return false;
		const enclosingStart = this.model.offsetAt(enclosing.getStartPosition());
		const enclosingEnd = this.model.offsetAt(enclosing.getEndPosition());
		const closeStart = this.model.offsetAt(closer.getStartPosition());
		const closeEnd = this.model.offsetAt(closer.getEndPosition());
		if (closeEnd !== enclosingEnd || closeStart < enclosingStart + entry.open.length) return false;
		if (this.model.getTextInRange(closer) !== entry.close) return false;
		const openEnd = this.model.positionAt(enclosingStart + entry.open.length);
		if (this.model.getTextInRange(Range.fromPositions(enclosing.getStartPosition(), openEnd)) !== entry.open) return false;
		return this.selections.selections.selections.some(selection => (
			Position.compare(selection.getStartPosition(), enclosing.getStartPosition()) > 0 &&
			Position.compare(selection.getEndPosition(), enclosing.getEndPosition()) < 0
		));
	}

	private disposeEntry(entry: AutoClosingEntry): void {
		entry.closeRange.dispose();
		entry.enclosingRange.dispose();
	}

}

function assertAction(model: TextModel, action: LanguageAutoClosingAction): void {
	if (typeof action !== "object" || action === null || typeof action.open !== "string" || action.open.length === 0 || typeof action.close !== "string" || action.close.length === 0) {
		throw new TypeError("Auto-closing action must contain non-empty open and close text");
	}
	const offsets = [action.enclosingStartOffset, action.closeStartOffset, action.closeEndOffset];
	if (offsets.some(offset => !Number.isSafeInteger(offset) || offset < 0)) {
		throw new RangeError("Auto-closing action offsets must be non-negative safe integers");
	}
	if (action.enclosingStartOffset + action.open.length !== action.closeStartOffset || action.closeStartOffset + action.close.length !== action.closeEndOffset) {
		throw new RangeError("Auto-closing action offsets must describe one empty pair");
	}
	const range = Range.fromPositions(model.positionAt(action.enclosingStartOffset), model.positionAt(action.closeEndOffset));
	if (model.getTextInRange(range) !== action.open + action.close) {
		throw new Error("Auto-closing action does not match the committed model text");
	}
}
