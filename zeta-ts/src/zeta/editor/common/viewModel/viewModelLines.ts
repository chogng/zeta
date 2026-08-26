import { Emitter, type Event } from '../../../base/common/event.js';
import { DisposableOwner, DisposableSlot, type IDisposable } from '../../../base/common/lifecycle.js';
import { isFiniteNumber, isPositiveSafeInteger } from '../../../base/common/numbers.js';
import { EditorLineWrapping, isWrappingIndent, WrappingIndent } from '../config/editorOptions.js';
import { type TextModel } from '../model/textModel.js';
import { type EditorViewportLineSource } from '../viewLayout/linesLayout.js';
import { EditorVisualLineProjection, type EditorVisualLine } from './modelLineProjection.js';

/** Supplies logical-line visibility without importing a browser feature. */
export interface EditorLineVisibilitySource {
	readonly onDidChange: Event<void>;
	isLineVisible(lineIndex: number): boolean;
}

/** Browser or worker implementation that turns one logical line into visual rows. */
export interface ILineBreaksComputer {
	computeLineBreaks(text: string, wrapWidth: number, wrappingIndent?: WrappingIndent): readonly number[];
	/** Optional extended result used to lay out wrapped continuation rows. */
	computeLineBreaksWithIndent?(text: string, wrapWidth: number, wrappingIndent: WrappingIndent): LineBreaksResult;
}

export interface LineBreaksResult {
	readonly breakColumns: readonly number[];
	readonly wrappedTextIndentWidth: number;
}

export interface ViewModelLinesOptions {
	readonly wrapping?: EditorLineWrapping;
	readonly wrapWidth?: number;
	readonly wrappingIndent?: WrappingIndent;
	/**
	 * Defers expensive initial soft-wrap measurement while preserving a usable
	 * one-row-per-logical-line projection until the complete result is ready.
	 */
	readonly initialWrappingMeasurement?: ViewModelLinesInitialMeasurementOptions;
	/** Optional logical-line visibility supplied by folding or another feature. */
	readonly visibilitySource?: EditorLineVisibilitySource;
}

/** Schedules a later, cancellable slice of initial visual-line measurement. */
export type ViewModelLinesMeasurementScheduler = (callback: () => void) => IDisposable;

/** Controls non-blocking initial measurement for a large wrapped document. */
export interface ViewModelLinesInitialMeasurementOptions {
	readonly initialLineCount?: number;
	readonly linesPerSlice?: number;
	readonly schedule: ViewModelLinesMeasurementScheduler;
}

interface ResolvedInitialMeasurement {
	readonly initialLineCount: number;
	readonly linesPerSlice: number;
	readonly schedule: ViewModelLinesMeasurementScheduler;
}

/**
 * Common view-model line collection for wrapping and hidden logical lines.
 *
 * This is the Stanza equivalent of VS Code's `viewModelLines.ts`: it owns the
 * model-versioned logical-to-visual mapping and combines visibility with the
 * wrapped rows. The browser supplies only the line-break computation policy.
 */
export class ViewModelLines extends DisposableOwner {
	private readonly changeEmitter = this.own(new Emitter<void>());
	private readonly lineCountChangeEmitter = this.own(new Emitter<void>());
	private readonly initialMeasurement: ResolvedInitialMeasurement | undefined;
	private readonly pendingMeasurement = this.own(new DisposableSlot<IDisposable>());
	private readonly visibilitySource: EditorLineVisibilitySource | undefined;
	private wrapping: EditorLineWrapping;
	private wrapWidth: number;
	private currentWrappingIndent: WrappingIndent;
	private wrappingProjection: EditorVisualLineProjection;
	private currentProjection: EditorVisualLineProjection;
	private projectionRevision = 0;
	private pendingBreaks: LineBreaksResult[] | undefined;
	private nextLineIndex = 0;
	private scanVersion = -1;

	readonly onDidChange: Event<void> = this.changeEmitter.event;
	readonly onDidChangeLineCount: Event<void> = this.lineCountChangeEmitter.event;
	readonly lineSource: EditorViewportLineSource;

	constructor(
		private readonly model: TextModel,
		private readonly lineBreaksComputer: ILineBreaksComputer,
		options: ViewModelLinesOptions = {},
	) {
		super();
		if (!lineBreaksComputer || typeof lineBreaksComputer.computeLineBreaks !== 'function') {
			throw new TypeError('Stanza view-model lines require a line-break computer');
		}
		this.visibilitySource = options.visibilitySource;
		this.wrapping = readWrapping(options.wrapping);
		this.wrapWidth = readWrapWidth(options.wrapWidth);
		this.currentWrappingIndent = readWrappingIndent(options.wrappingIndent);
		this.initialMeasurement = readInitialMeasurement(options.initialWrappingMeasurement);
		this.wrappingProjection = this.usesInitialMeasurement()
			? EditorVisualLineProjection.identity(this.model)
			: this.createWrappingProjection();
		this.currentProjection = this.createVisibleProjection(this.wrappingProjection);
		const projection = this;
		this.lineSource = Object.freeze({
			get lineCount(): number {
				return projection.currentProjection.visualLineCount;
			},
			onDidChange: this.onDidChange,
		});
		if (this.usesInitialMeasurement()) this.startInitialMeasurement();
		this.own(this.model.onDidChange(() => this.refresh()));
		if (this.visibilitySource) this.own(this.visibilitySource.onDidChange(() => this.rebuildVisibleProjection()));
	}

	get textModel(): TextModel {
		return this.model;
	}

	/** The current wrapping-plus-visibility projection. */
	get projection(): EditorVisualLineProjection {
		return this.currentProjection;
	}

	get lineCount(): number {
		return this.currentProjection.visualLineCount;
	}

	get wrappingIndent(): WrappingIndent {
		return this.currentWrappingIndent;
	}

	get revision(): number {
		return this.projectionRevision;
	}

	/** Whether the current soft-wrap projection includes every model line. */
	get complete(): boolean {
		return !this.pendingBreaks || this.nextLineIndex >= this.model.lineCount;
	}

	ensureCurrent(): EditorVisualLineProjection {
		if (this.wrappingProjection.modelVersion !== this.model.version) this.refresh();
		return this.currentProjection;
	}

	setWrapping(wrapping: EditorLineWrapping): void {
		const next = readWrapping(wrapping);
		if (next === this.wrapping) return;
		this.wrapping = next;
		this.refresh();
	}

	setWrapWidth(width: number): void {
		const next = readWrapWidth(width);
		if (next === this.wrapWidth) return;
		this.wrapWidth = next;
		this.refresh();
	}

	setWrappingIndent(wrappingIndent: WrappingIndent): void {
		const next = readWrappingIndent(wrappingIndent);
		if (next === this.currentWrappingIndent) return;
		this.currentWrappingIndent = next;
		this.refresh();
	}

	private refresh(): void {
		if (this.usesInitialMeasurement()) this.startInitialMeasurement();
		else this.rebuild();
	}

	private rebuild(): void {
		this.pendingMeasurement.clear();
		this.pendingBreaks = undefined;
		this.nextLineIndex = this.model.lineCount;
		this.scanVersion = this.model.version;
		this.replaceWrappingProjection(this.createWrappingProjection());
	}

	private startInitialMeasurement(): void {
		const options = this.initialMeasurement;
		if (!options) return;
		this.pendingMeasurement.clear();
		this.scanVersion = this.model.version;
		this.nextLineIndex = 0;
		this.pendingBreaks = Array.from(
			{ length: this.model.lineCount },
			(_, lineIndex) => Object.freeze({
				breakColumns: Object.freeze([this.model.getLineContent(lineIndex).length]),
				wrappedTextIndentWidth: 0,
			}),
		);
		this.measureNextSlice(options.initialLineCount);
		this.replaceWrappingProjection(this.createProjectionFromPendingBreaks());
		this.scheduleNextSlice();
	}

	private scheduleNextSlice(): void {
		const options = this.initialMeasurement;
		if (!options || this.complete) return;
		this.pendingMeasurement.replace(options.schedule(() => {
			this.pendingMeasurement.clear();
			if (this.scanVersion !== this.model.version) {
				this.startInitialMeasurement();
				return;
			}
			this.measureNextSlice(options.linesPerSlice);
			if (this.complete) this.replaceWrappingProjection(this.createProjectionFromPendingBreaks());
			this.scheduleNextSlice();
		}));
	}

	private measureNextSlice(lineCount: number): void {
		const breaks = this.pendingBreaks;
		if (!breaks) return;
		const endLineIndex = Math.min(this.model.lineCount, this.nextLineIndex + lineCount);
		for (; this.nextLineIndex < endLineIndex; this.nextLineIndex += 1) {
			breaks[this.nextLineIndex] = computeLineBreaksForLine(
				this.lineBreaksComputer,
				this.model.getLineContent(this.nextLineIndex),
				this.wrapWidth,
				this.currentWrappingIndent,
			);
		}
	}

	private replaceWrappingProjection(next: EditorVisualLineProjection): void {
		const previousLineCount = this.currentProjection.visualLineCount;
		this.wrappingProjection = next;
		this.currentProjection = this.createVisibleProjection(next);
		this.projectionRevision += 1;
		if (this.currentProjection.visualLineCount !== previousLineCount) this.lineCountChangeEmitter.fire();
		this.changeEmitter.fire();
	}

	private rebuildVisibleProjection(): void {
		if (this.wrappingProjection.modelVersion !== this.model.version) {
			this.refresh();
			return;
		}
		const previousLineCount = this.currentProjection.visualLineCount;
		this.currentProjection = this.createVisibleProjection(this.wrappingProjection);
		this.projectionRevision += 1;
		if (this.currentProjection.visualLineCount !== previousLineCount) this.lineCountChangeEmitter.fire();
		this.changeEmitter.fire();
	}

	private createWrappingProjection(): EditorVisualLineProjection {
		if (this.wrapping === EditorLineWrapping.Off || this.wrapWidth === 0) {
			return EditorVisualLineProjection.identity(this.model);
		}
		const breakColumnsByLine: number[][] = [];
		const wrappedTextIndentWidthsByLine: number[] = [];
		for (let lineIndex = 0; lineIndex < this.model.lineCount; lineIndex += 1) {
			const text = this.model.getLineContent(lineIndex);
			const result = computeLineBreaksForLine(this.lineBreaksComputer, text, this.wrapWidth, this.currentWrappingIndent);
			breakColumnsByLine.push([...result.breakColumns]);
			wrappedTextIndentWidthsByLine.push(result.wrappedTextIndentWidth);
		}
		return EditorVisualLineProjection.fromBreakColumns(this.model, breakColumnsByLine, wrappedTextIndentWidthsByLine);
	}

	private createProjectionFromPendingBreaks(): EditorVisualLineProjection {
		const breaks = this.pendingBreaks;
		if (!breaks) throw new Error('Stanza visual-line measurement is not active');
		return EditorVisualLineProjection.fromBreakColumns(
			this.model,
			breaks.map(result => result.breakColumns),
			breaks.map(result => result.wrappedTextIndentWidth),
		);
	}

	private createVisibleProjection(source: EditorVisualLineProjection): EditorVisualLineProjection {
		if (!this.visibilitySource) return source;
		const visibleLogicalLines = Object.freeze(Array.from(
			{ length: source.logicalLineCount },
			(_, lineIndex) => this.visibilitySource!.isLineVisible(lineIndex),
		));
		const lines = source.lines.filter(line => visibleLogicalLines[line.logicalLineIndex]);
		const anchors = createVisualLineAnchors(source, visibleLogicalLines, lines);
		return EditorVisualLineProjection.fromVisibleLines(source.modelVersion, source.logicalLineCount, lines, anchors);
	}

	private usesInitialMeasurement(): boolean {
		return this.initialMeasurement !== undefined &&
			this.wrapping === EditorLineWrapping.On &&
			this.wrapWidth > 0;
	}
}

function createVisualLineAnchors(
	source: EditorVisualLineProjection,
	visibleLogicalLines: readonly boolean[],
	lines: readonly EditorVisualLine[],
): readonly number[] {
	const first = Array.from({ length: source.logicalLineCount }, () => -1);
	const last = Array.from({ length: source.logicalLineCount }, () => -1);
	for (let visualLineIndex = 0; visualLineIndex < lines.length; visualLineIndex += 1) {
		const line = lines[visualLineIndex]!;
		if (line.firstForLogicalLine) first[line.logicalLineIndex] = visualLineIndex;
		if (line.lastForLogicalLine) last[line.logicalLineIndex] = visualLineIndex;
	}
	let previousVisible = -1;
	const anchors: number[] = [];
	for (let logicalLineIndex = 0; logicalLineIndex < source.logicalLineCount; logicalLineIndex += 1) {
		if (visibleLogicalLines[logicalLineIndex]) {
			previousVisible = last[logicalLineIndex]!;
			anchors.push(first[logicalLineIndex]!);
		} else {
			if (previousVisible < 0) throw new Error('A visible-line projection must retain the first logical line');
			anchors.push(previousVisible);
		}
	}
	return Object.freeze(anchors);
}

function readWrapping(value: EditorLineWrapping | undefined): EditorLineWrapping {
	const wrapping = value ?? EditorLineWrapping.Off;
	if (!Object.values(EditorLineWrapping).includes(wrapping)) {
		throw new TypeError('Unknown Stanza editor line wrapping mode');
	}
	return wrapping;
}

function readWrappingIndent(value: WrappingIndent | undefined): WrappingIndent {
	const wrappingIndent = value ?? WrappingIndent.Same;
	if (!isWrappingIndent(wrappingIndent)) {
		throw new TypeError('Unknown Stanza wrapping indent mode');
	}
	return wrappingIndent;
}

function computeLineBreaksForLine(computer: ILineBreaksComputer, text: string, wrapWidth: number, wrappingIndent: WrappingIndent): LineBreaksResult {
	const extended = computer.computeLineBreaksWithIndent;
	if (extended) {
		const result = extended.call(computer, text, wrapWidth, wrappingIndent);
		if (!result || !Array.isArray(result.breakColumns)) {
			throw new TypeError('Stanza line-break computer must return break columns');
		}
		if (!isFiniteNumber(result.wrappedTextIndentWidth) || result.wrappedTextIndentWidth < 0) {
			throw new RangeError('Stanza line-break computer must return a finite non-negative wrapped-text indent width');
		}
		return Object.freeze({
			breakColumns: Object.freeze([...result.breakColumns]),
			wrappedTextIndentWidth: result.wrappedTextIndentWidth,
		});
	}
	return Object.freeze({
		breakColumns: Object.freeze([...computer.computeLineBreaks(text, wrapWidth, wrappingIndent)]),
		wrappedTextIndentWidth: 0,
	});
}

function readWrapWidth(value: number | undefined): number {
	const width = value ?? 0;
	if (!isFiniteNumber(width) || width < 0) {
		throw new RangeError('Stanza editor wrap width must be finite and non-negative');
	}
	return width;
}

function readInitialMeasurement(value: ViewModelLinesInitialMeasurementOptions | undefined): ResolvedInitialMeasurement | undefined {
	if (value === undefined) return undefined;
	if (!value || typeof value.schedule !== 'function') {
		throw new TypeError('Stanza initial visual-line measurement requires a scheduler');
	}
	const initialLineCount = value.initialLineCount ?? 512;
	const linesPerSlice = value.linesPerSlice ?? initialLineCount;
	if (!isPositiveSafeInteger(initialLineCount)) {
		throw new RangeError('Stanza initial visual-line measurement count must be a positive safe integer');
	}
	if (!isPositiveSafeInteger(linesPerSlice)) {
		throw new RangeError('Stanza visual-line measurement slice size must be a positive safe integer');
	}
	return Object.freeze({ initialLineCount, linesPerSlice, schedule: value.schedule });
}
