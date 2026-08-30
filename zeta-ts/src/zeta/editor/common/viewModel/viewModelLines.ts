import { Emitter, type Event } from '../../../base/common/event.js';
import { Disposable, MutableDisposable, type IDisposable } from '../../../base/common/lifecycle.js';
import { isFiniteNumber, isPositiveSafeInteger } from '../../../base/common/numbers.js';
import { EditorLineWrapping, isWrappingIndent, WrappingIndent } from '../config/editorOptions.js';
import { type FontInfo } from '../config/fontInfo.js';
import { type ICoordinatesConverter } from '../coordinatesConverter.js';
import { type ICursorSimpleModel } from '../cursorCommon.js';
import { Position } from '../core/position.js';
import { Range } from '../core/range.js';
import { PositionAffinity } from '../model.js';
import { type TextModel } from '../model/textModel.js';
import { type ILineBreaksComputerContext, type ILineBreaksComputerFactory, type ModelLineProjectionData } from '../modelLineProjectionData.js';
import { type EditorViewportLineSource } from './editorViewportContracts.js';
import { EditorVisualLineProjection, type EditorVisualLine } from './modelLineProjection.js';

/** Supplies logical-line visibility without importing a browser feature. */
export interface EditorLineVisibilitySource {
	readonly onDidChange: Event<void>;
	isLineVisible(lineIndex: number): boolean;
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
export class ViewModelLines extends Disposable implements ICursorSimpleModel {
	private readonly changeEmitter = this._register(new Emitter<void>());
	private readonly lineCountChangeEmitter = this._register(new Emitter<void>());
	private readonly initialMeasurement: ResolvedInitialMeasurement | undefined;
	private readonly pendingMeasurement = this._register(new MutableDisposable<IDisposable>());
	private readonly visibilitySource: EditorLineVisibilitySource | undefined;
	private wrapping: EditorLineWrapping;
	private wrapWidth: number;
	private currentWrappingIndent: WrappingIndent;
	private wrappingProjection: EditorVisualLineProjection;
	private currentProjection: EditorVisualLineProjection;
	private projectionRevision = 0;
	private pendingBreaks: Array<ModelLineProjectionData | null> | undefined;
	private nextLineIndex = 0;
	private scanVersion = -1;

	readonly onDidChange: Event<void> = this.changeEmitter.event;
	readonly onDidChangeLineCount: Event<void> = this.lineCountChangeEmitter.event;
	readonly lineSource: EditorViewportLineSource;

	constructor(
		private readonly model: TextModel,
		private readonly lineBreaksComputerFactory: ILineBreaksComputerFactory,
		private readonly fontInfo: FontInfo,
		private readonly tabSize: number,
		options: ViewModelLinesOptions = {},
	) {
		super();
		if (!lineBreaksComputerFactory || typeof lineBreaksComputerFactory.createLineBreaksComputer !== 'function') {
			throw new TypeError('Editor view-model lines require a line-break computer factory');
		}
		if (!fontInfo || !isFiniteNumber(fontInfo.typicalHalfwidthCharacterWidth) || fontInfo.typicalHalfwidthCharacterWidth <= 0) throw new TypeError('Editor view-model lines require measured font information');
		if (!Number.isSafeInteger(tabSize) || tabSize < 1) throw new RangeError('Editor view-model tab size must be a positive safe integer');
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
		this._register(this.model.onDidChangeContent(() => this.refresh()));
		if (this.visibilitySource) this._register(this.visibilitySource.onDidChange(() => this.rebuildVisibleProjection()));
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

	createCoordinatesConverter(): ICoordinatesConverter {
		return new ViewModelCoordinatesConverter(this.model, this);
	}

	getLineCount(): number {
		return this.ensureCurrent().visualLineCount;
	}

	getLineContent(lineNumber: number): string {
		const line = this.getVisualLine(lineNumber);
		return this.model.getLineContent(line.logicalLineIndex + 1).slice(line.startColumn, line.endColumn);
	}

	getLineMinColumn(_lineNumber: number): number {
		return 1;
	}

	getLineMaxColumn(lineNumber: number): number {
		return this.getLineContent(lineNumber).length + 1;
	}

	getLineFirstNonWhitespaceColumn(lineNumber: number): number {
		const index = this.getLineContent(lineNumber).search(/\S/u);
		return index < 0 ? 0 : index + 1;
	}

	getLineLastNonWhitespaceColumn(lineNumber: number): number {
		const content = this.getLineContent(lineNumber);
		for (let index = content.length - 1; index >= 0; index -= 1) {
			if (/\S/u.test(content[index]!)) return index + 2;
		}
		return 0;
	}

	normalizePosition(position: Position, _affinity: PositionAffinity): Position {
		const lineNumber = Math.min(Math.max(position.lineNumber, 1), this.getLineCount());
		return new Position(lineNumber, Math.min(Math.max(position.column, 1), this.getLineMaxColumn(lineNumber)));
	}

	getLineIndentColumn(lineNumber: number): number {
		const firstNonWhitespaceColumn = this.getLineFirstNonWhitespaceColumn(lineNumber);
		return firstNonWhitespaceColumn === 0 ? this.getLineMaxColumn(lineNumber) : firstNonWhitespaceColumn;
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
		this.pendingBreaks = Array.from({ length: this.model.lineCount }, () => null);
		this.measureNextSlice(options.initialLineCount);
		this.replaceWrappingProjection(this.createProjectionFromPendingBreaks());
		this.scheduleNextSlice();
	}

	private scheduleNextSlice(): void {
		const options = this.initialMeasurement;
		if (!options || this.complete) return;
		this.pendingMeasurement.value = options.schedule(() => {
			this.pendingMeasurement.clear();
			if (this.scanVersion !== this.model.version) {
				this.startInitialMeasurement();
				return;
			}
			this.measureNextSlice(options.linesPerSlice);
			if (this.complete) this.replaceWrappingProjection(this.createProjectionFromPendingBreaks());
			this.scheduleNextSlice();
		});
	}

	private measureNextSlice(lineCount: number): void {
		const breaks = this.pendingBreaks;
		if (!breaks) return;
		const endLineIndex = Math.min(this.model.lineCount, this.nextLineIndex + lineCount);
		const computer = this.createLineBreaksComputer();
		for (let lineIndex = this.nextLineIndex; lineIndex < endLineIndex; lineIndex += 1) computer.addRequest(lineIndex + 1, breaks[lineIndex] ?? null);
		const measured = computer.finalize();
		if (measured.length !== endLineIndex - this.nextLineIndex) throw new Error('Line-break computer returned a result count different from its requests');
		for (const result of measured) breaks[this.nextLineIndex++] = result;
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
		const computer = this.createLineBreaksComputer();
		for (let lineNumber = 1; lineNumber <= this.model.lineCount; lineNumber += 1) computer.addRequest(lineNumber, null);
		return this.createProjection(computer.finalize());
	}

	private createProjectionFromPendingBreaks(): EditorVisualLineProjection {
		const breaks = this.pendingBreaks;
		if (!breaks) throw new Error('Stanza visual-line measurement is not active');
		return this.createProjection(breaks);
	}

	private createProjection(breaks: readonly (ModelLineProjectionData | null)[]): EditorVisualLineProjection {
		if (breaks.length !== this.model.lineCount) throw new Error('Line-break data must match the model line count');
		return EditorVisualLineProjection.fromBreakColumns(this.model, breaks.map((result, lineIndex) => (
			result?.breakOffsets ?? [this.model.getLineContent(lineIndex + 1).length]
		)), breaks.map(result => (result?.wrappedTextIndentLength ?? 0) * this.fontInfo.spaceWidth));
	}

	private createLineBreaksComputer() {
		const context: ILineBreaksComputerContext = {
			getLineContent: lineNumber => this.model.getLineContent(lineNumber),
			getLineInjectedText: _lineNumber => null,
		};
		return this.lineBreaksComputerFactory.createLineBreaksComputer(
			context,
			this.fontInfo,
			this.tabSize,
			Math.max(0, Math.floor(this.wrapWidth / this.fontInfo.typicalHalfwidthCharacterWidth)),
			this.currentWrappingIndent,
			'normal',
			false,
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

	private getVisualLine(lineNumber: number): EditorVisualLine {
		const line = this.ensureCurrent().lineAt(lineNumber - 1);
		if (!line) throw new RangeError('View line number is outside the visual projection');
		return line;
	}
}

class ViewModelCoordinatesConverter implements ICoordinatesConverter {
	constructor(
		private readonly model: TextModel,
		private readonly lines: ViewModelLines,
	) {}

	public convertViewPositionToModelPosition(viewPosition: Position): Position {
		const position = this.lines.normalizePosition(viewPosition, PositionAffinity.None);
		const line = this.lines.projection.lineAt(position.lineNumber - 1)!;
		return this.model.validatePosition(new Position(line.logicalLineIndex + 1, line.startColumn + position.column));
	}

	public convertViewRangeToModelRange(viewRange: Range): Range {
		const range = this.validateViewRange(viewRange, this.model.getFullModelRange());
		return Range.fromPositions(
			this.convertViewPositionToModelPosition(range.getStartPosition()),
			this.convertViewPositionToModelPosition(range.getEndPosition()),
		);
	}

	public validateViewPosition(viewPosition: Position, expectedModelPosition: Position): Position {
		const expected = this.convertModelPositionToViewPosition(expectedModelPosition);
		return expected.equals(viewPosition) ? expected : this.lines.normalizePosition(viewPosition, PositionAffinity.None);
	}

	public validateViewRange(viewRange: Range, expectedModelRange: Range): Range {
		const expected = this.convertModelRangeToViewRange(expectedModelRange);
		if (expected.equalsRange(viewRange)) return expected;
		return Range.fromPositions(
			this.lines.normalizePosition(viewRange.getStartPosition(), PositionAffinity.Left),
			this.lines.normalizePosition(viewRange.getEndPosition(), PositionAffinity.Right),
		);
	}

	public convertModelPositionToViewPosition(modelPosition: Position, affinity: PositionAffinity = PositionAffinity.None): Position {
		const position = this.model.validatePosition(modelPosition);
		const projection = this.lines.ensureCurrent();
		let visualLineIndex = projection.visualLineIndexAt(position);
		let line = projection.lineAt(visualLineIndex)!;
		if (affinity === PositionAffinity.Left && position.column - 1 === line.startColumn && visualLineIndex > 0) {
			const previous = projection.lineAt(visualLineIndex - 1)!;
			if (previous.logicalLineIndex === line.logicalLineIndex) {
				visualLineIndex -= 1;
				line = previous;
			}
		}
		return new Position(visualLineIndex + 1, position.column - line.startColumn);
	}

	public convertModelRangeToViewRange(modelRange: Range, affinity: PositionAffinity = PositionAffinity.None): Range {
		const range = this.model.validateRange(modelRange);
		return Range.fromPositions(
			this.convertModelPositionToViewPosition(range.getStartPosition(), affinity),
			this.convertModelPositionToViewPosition(range.getEndPosition(), affinity),
		);
	}

	public modelPositionIsVisible(modelPosition: Position): boolean {
		if (modelPosition.lineNumber < 1 || modelPosition.lineNumber > this.model.getLineCount()) return false;
		const projection = this.lines.ensureCurrent();
		return projection.lineAt(projection.visualLineIndexAt(this.model.validatePosition(modelPosition)))?.logicalLineIndex === modelPosition.lineNumber - 1;
	}

	public getModelLineViewLineCount(modelLineNumber: number): number {
		if (modelLineNumber < 1 || modelLineNumber > this.model.getLineCount()) return 1;
		return this.lines.ensureCurrent().lines.filter(line => line.logicalLineIndex === modelLineNumber - 1).length || 1;
	}

	public getViewLineNumberOfModelPosition(modelLineNumber: number, modelColumn: number): number {
		return this.convertModelPositionToViewPosition(new Position(modelLineNumber, modelColumn)).lineNumber;
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
