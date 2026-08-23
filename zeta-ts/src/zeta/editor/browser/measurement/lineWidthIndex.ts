import { CharCode } from "../../../base/common/charCode.js";
import { Emitter, type Event } from "../../../base/common/event.js";
import { DisposableOwner, DisposableSlot, type IDisposable } from "../../../base/common/lifecycle.js";
import { type TextModelChange } from "../../common/core/text.js";
import { type TextModel } from "../../common/model/textModel.js";
import { type TextMeasurer } from "../../common/viewModel/textMeasurer.js";

interface AffectedLineGroup {
	readonly oldStartLineIndex: number;
	oldEndLineIndex: number;
	lineDelta: number;
}

interface MeasuredLineGroup extends AffectedLineGroup {
	readonly newWidths: readonly number[];
}

/** Schedules a later, cancellable portion of an initial line-width scan. */
export type LineWidthMeasurementScheduler = (callback: () => void) => IDisposable;

/**
 * Controls a non-blocking initial width scan for a large, non-wrapped model.
 *
 * The synchronous first slice supplies a deterministic lower bound immediately.
 * Later slices monotonically refine it, and `onDidChange` fires whenever that
 * bound changes or the scan has to restart after an edit.
 */
export interface LineWidthInitialMeasurementOptions {
	readonly initialLineCount?: number;
	readonly linesPerSlice?: number;
	/** Optional startup scan cap; later visible lines are measured on demand. */
	readonly maximumMeasuredLineCount?: number;
	readonly schedule: LineWidthMeasurementScheduler;
}

export interface LineWidthIndexOptions {
	readonly initialMeasurement?: LineWidthInitialMeasurementOptions;
}

interface ResolvedInitialMeasurement {
	readonly initialLineCount: number;
	readonly linesPerSlice: number;
	readonly maximumMeasuredLineCount: number;
	readonly schedule: LineWidthMeasurementScheduler;
}

/** Viewport-owned width index used to bound horizontal layout work. */
export class LineWidthIndex extends DisposableOwner {
	private widths: number[] = [];
	private readonly widthCounts = new Map<number, number>();
	private readonly changeEmitter = this.own(new Emitter<void>());
	private readonly pendingMeasurement = this.own(new DisposableSlot<IDisposable>());
	private readonly observedLineIndexes = new Set<number>();
	private readonly initialMeasurement: ResolvedInitialMeasurement | undefined;
	private maximumWidth = 0;
	private nextLineIndex = 0;
	private scanVersion = 0;

	constructor(
		private readonly model: TextModel,
		private readonly measurer: TextMeasurer,
		options: LineWidthIndexOptions = {},
	) {
		super();
		this.initialMeasurement = readInitialMeasurement(options.initialMeasurement);
		if (this.initialMeasurement) this.startInitialMeasurement();
		else this.rebuild();
	}

	readonly onDidChange: Event<void> = this.changeEmitter.event;

	get maximumLineWidth(): number {
		return this.maximumWidth;
	}

	/** Whether every current model line has been included in the width index. */
	get complete(): boolean {
		return this.nextLineIndex >= this.model.lineCount;
	}

	/** Rebuilds with this index's configured initial-measurement policy. */
	refresh(): void {
		if (this.initialMeasurement) this.startInitialMeasurement();
		else this.rebuild();
	}

	rebuild(): void {
		this.pendingMeasurement.clear();
		const previousMaximum = this.maximumWidth;
		this.widths = [];
		this.widthCounts.clear();
		this.observedLineIndexes.clear();
		this.maximumWidth = 0;
		for (let lineIndex = 0; lineIndex < this.model.lineCount; lineIndex++) {
			const width = this.measure(lineIndex);
			this.widths.push(width);
			this.addWidth(width);
		}
		this.nextLineIndex = this.model.lineCount;
		this.scanVersion = this.model.version;
		if (this.maximumWidth !== previousMaximum) this.changeEmitter.fire();
	}

	applyModelChange(change: TextModelChange): void {
		if (this.initialMeasurement && !this.complete) {
			this.startInitialMeasurement();
			return;
		}
		const previousMaximum = this.maximumWidth;
		const groups = groupAffectedLines(change);
		let cumulativeLineDelta = 0;
		const measured: MeasuredLineGroup[] = [];
		for (const group of groups) {
			const oldLineCount =
				group.oldEndLineIndex - group.oldStartLineIndex + 1;
			const newLineCount = oldLineCount + group.lineDelta;
			const newStartLineIndex =
				group.oldStartLineIndex + cumulativeLineDelta;
			const newWidths = Array.from(
				{ length: newLineCount },
				(_, index) => this.measure(newStartLineIndex + index),
			);
			measured.push({ ...group, newWidths });
			cumulativeLineDelta += group.lineDelta;
		}

		for (let index = measured.length - 1; index >= 0; index--) {
			const group = measured[index];
			if (!group) continue;
			const oldLineCount =
				group.oldEndLineIndex - group.oldStartLineIndex + 1;
			const removed = this.widths.splice(
				group.oldStartLineIndex,
				oldLineCount,
				...group.newWidths,
			);
			for (const width of removed) this.removeWidth(width);
			for (const width of group.newWidths) this.addWidth(width);
		}

		if (this.widths.length !== this.model.lineCount) {
			this.rebuild();
			return;
		}
		if (!this.widthCounts.has(this.maximumWidth)) {
			this.maximumWidth = 0;
			for (const width of this.widthCounts.keys()) {
				this.maximumWidth = Math.max(this.maximumWidth, width);
			}
		}
		this.nextLineIndex = this.model.lineCount;
		this.scanVersion = this.model.version;
		if (this.maximumWidth !== previousMaximum) this.changeEmitter.fire();
	}

	/** Measures newly visible lines that lie beyond a bounded initial scan. */
	observeLines(lineIndexes: readonly number[]): void {
		const previousMaximum = this.maximumWidth;
		for (const lineIndex of lineIndexes) {
			if (!Number.isSafeInteger(lineIndex) || lineIndex < 0 || lineIndex >= this.model.lineCount) {
				throw new RangeError("Observed Stanza line index is outside the text model");
			}
			if (lineIndex < this.nextLineIndex || this.observedLineIndexes.has(lineIndex)) continue;
			this.observedLineIndexes.add(lineIndex);
			this.maximumWidth = Math.max(this.maximumWidth, this.measure(lineIndex));
		}
		if (this.maximumWidth !== previousMaximum) this.changeEmitter.fire();
	}

	private startInitialMeasurement(): void {
		const options = this.initialMeasurement;
		if (!options) return;
		const previousMaximum = this.maximumWidth;
		this.pendingMeasurement.clear();
		this.widths = [];
		this.widthCounts.clear();
		this.observedLineIndexes.clear();
		this.maximumWidth = 0;
		this.nextLineIndex = 0;
		this.scanVersion = this.model.version;
		this.measureNextSlice(options.initialLineCount);
		if (this.maximumWidth !== previousMaximum) this.changeEmitter.fire();
		this.scheduleNextSlice();
	}

	private scheduleNextSlice(): void {
		const options = this.initialMeasurement;
		if (!options || this.nextLineIndex >= this.initialScanLineCount) return;
		this.pendingMeasurement.replace(options.schedule(() => {
			this.pendingMeasurement.clear();
			if (this.scanVersion !== this.model.version) {
				this.startInitialMeasurement();
				return;
			}
			const previousMaximum = this.maximumWidth;
			this.measureNextSlice(options.linesPerSlice);
			if (this.maximumWidth !== previousMaximum) this.changeEmitter.fire();
			this.scheduleNextSlice();
		}));
	}

	private measureNextSlice(lineCount: number): void {
		const endLineIndex = Math.min(this.initialScanLineCount, this.nextLineIndex + lineCount);
		for (; this.nextLineIndex < endLineIndex; this.nextLineIndex += 1) {
			const width = this.measure(this.nextLineIndex);
			this.widths.push(width);
			this.addWidth(width);
		}
	}

	private get initialScanLineCount(): number {
		return Math.min(this.model.lineCount, this.initialMeasurement?.maximumMeasuredLineCount ?? this.model.lineCount);
	}

	private measure(lineIndex: number): number {
		const width = this.measurer.measureLineWidth(
			this.model.getLineContent(lineIndex),
		);
		if (!Number.isFinite(width) || width < 0) {
			throw new RangeError("Stanza line width must be finite and non-negative");
		}
		return width;
	}

	private addWidth(width: number): void {
		this.widthCounts.set(width, (this.widthCounts.get(width) ?? 0) + 1);
		this.maximumWidth = Math.max(this.maximumWidth, width);
	}

	private removeWidth(width: number): void {
		const count = this.widthCounts.get(width);
		if (count === undefined) {
			throw new Error("Stanza line width index is inconsistent");
		}
		if (count === 1) this.widthCounts.delete(width);
		else this.widthCounts.set(width, count - 1);
	}
}

function readInitialMeasurement(value: LineWidthInitialMeasurementOptions | undefined): ResolvedInitialMeasurement | undefined {
	if (value === undefined) return undefined;
	if (!value || typeof value.schedule !== "function") {
		throw new TypeError("Stanza initial line measurement requires a scheduler");
	}
	const initialLineCount = value.initialLineCount ?? 512;
	const linesPerSlice = value.linesPerSlice ?? initialLineCount;
	const maximumMeasuredLineCount = value.maximumMeasuredLineCount ?? Number.MAX_SAFE_INTEGER;
	if (!Number.isSafeInteger(initialLineCount) || initialLineCount <= 0) {
		throw new RangeError("Stanza initial line measurement count must be a positive safe integer");
	}
	if (!Number.isSafeInteger(linesPerSlice) || linesPerSlice <= 0) {
		throw new RangeError("Stanza line measurement slice size must be a positive safe integer");
	}
	if (!Number.isSafeInteger(maximumMeasuredLineCount) || maximumMeasuredLineCount <= 0) {
		throw new RangeError("Stanza maximum initial line measurement count must be a positive safe integer");
	}
	return Object.freeze({ initialLineCount: Math.min(initialLineCount, maximumMeasuredLineCount), linesPerSlice, maximumMeasuredLineCount, schedule: value.schedule });
}

function groupAffectedLines(
	change: TextModelChange,
): AffectedLineGroup[] {
	const effects = change.changes
		.map((contentChange) => ({
			oldStartLineIndex: contentChange.range.start.lineIndex,
			oldEndLineIndex: contentChange.range.end.lineIndex,
			lineDelta:
				lineFeedCount(contentChange.text) -
				(
					contentChange.range.end.lineIndex -
					contentChange.range.start.lineIndex
				),
		}))
		.sort((left, right) =>
			left.oldStartLineIndex - right.oldStartLineIndex ||
			left.oldEndLineIndex - right.oldEndLineIndex);
	const groups: AffectedLineGroup[] = [];
	for (const effect of effects) {
		const previous = groups.at(-1);
		if (
			previous &&
			effect.oldStartLineIndex <= previous.oldEndLineIndex
		) {
			previous.oldEndLineIndex = Math.max(
				previous.oldEndLineIndex,
				effect.oldEndLineIndex,
			);
			previous.lineDelta += effect.lineDelta;
		} else {
			groups.push({ ...effect });
		}
	}
	return groups;
}

function lineFeedCount(text: string): number {
	let count = 0;
	for (let index = 0; index < text.length; index++) {
		if (text.charCodeAt(index) === CharCode.LineFeed) count++;
	}
	return count;
}
