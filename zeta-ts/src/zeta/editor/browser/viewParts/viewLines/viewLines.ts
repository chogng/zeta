import './viewLines.css';
import { FastDomNode } from '../../../../base/browser/fastDomNode.js';
import { CharCode } from '../../../../base/common/charCode.js';
import { Emitter, type Event } from '../../../../base/common/event.js';
import { Disposable, MutableDisposable, type IDisposable } from '../../../../base/common/lifecycle.js';
import { type EditorVisualLine, type EditorVisualLineProjection } from '../../../common/viewModel/modelLineProjection.js';
import { type EditorLineRange } from '../../../common/viewModel/editorViewportContracts.js';
import { Position } from '../../../common/core/position.js';
import { type Range } from '../../../common/core/range.js';
import { type TextModelChange } from '../../../common/core/textChange.js';
import { type ViewportData } from '../../../common/viewLayout/viewLinesViewportData.js';
import { type TextModel } from '../../../common/model/textModel.js';
import { type TextMeasurer } from '../../../common/viewModel/textMeasurer.js';
import { type IEditorConfiguration } from '../../../common/config/editorConfiguration.js';
import { type ColorScheme } from '../../../../platform/theme/common/theme.js';
import { type ViewContext } from '../../../common/viewModel/viewContext.js';
import { type ViewConfigurationChangedEvent, type ViewCursorStateChangedEvent, type ViewDecorationsChangedEvent, type ViewFlushedEvent, type ViewLinesChangedEvent, type ViewLinesDeletedEvent, type ViewLinesInsertedEvent, type ViewScrollChangedEvent, type ViewThemeChangedEvent, type ViewTokensChangedEvent, type ViewZonesChangedEvent } from '../../../common/viewEvents.js';
import { ViewLine, type BracketColorizationSource, type ResolvedSemanticToken, type SemanticTokenSource } from './viewLine.js';
import { DomReadingContext } from './domReadingContext.js';
import { ViewLineOptions } from './viewLineOptions.js';
import { ViewLayer } from '../../view/viewLayer.js';
import { FloatHorizontalRange, HorizontalPosition, HorizontalRange, type IViewLines, LineVisibleRanges, type RestrictedRenderingContext, type VisibleRanges } from '../../view/renderingContext.js';
import { ViewPart } from '../../view/viewPart.js';

export interface ViewLinesOptions {
	readonly host: HTMLElement;
	readonly model: TextModel;
	readonly readVisualProjection: () => EditorVisualLineProjection;
	readonly readProjectionRevision: () => number;
	readonly semanticTokenSource: SemanticTokenSource | undefined;
	readonly bracketColorizationSource: BracketColorizationSource | undefined;
	readonly configuration: IEditorConfiguration;
	readonly themeType: ColorScheme;
	readonly tabSize: number;
	readonly typicalHalfwidthCharacterWidth: number;
	readonly readGpuLineIndexes?: () => ReadonlySet<number>;
}

/** Projects text and semantic tokens into the generic virtualized ViewLayer. */
export class ViewLines extends ViewPart implements IViewLines {
	public readonly domNode: FastDomNode<HTMLDivElement>;
	private readonly model: TextModel;
	private readonly readVisualProjection: () => EditorVisualLineProjection;
	private readonly semanticTokenSource: SemanticTokenSource | undefined;
	private readonly bracketColorizationSource: BracketColorizationSource | undefined;
	private readonly _visibleLines: ViewLayer<ViewLine>;
	private readonly _typicalHalfwidthCharacterWidth: number;
	private readonly readGpuLineIndexes: () => ReadonlySet<number>;
	private _viewLineOptions: ViewLineOptions;
	private _maxLineWidth = 0;

	constructor(context: ViewContext, options: ViewLinesOptions) {
		super(context);
		this.model = options.model;
		this.readVisualProjection = options.readVisualProjection;
		this.semanticTokenSource = options.semanticTokenSource;
		this.bracketColorizationSource = options.bracketColorizationSource;
		this._viewLineOptions = new ViewLineOptions(options.configuration, options.themeType);
		if (!Number.isSafeInteger(options.tabSize) || options.tabSize < 1) throw new RangeError('Stanza view-line tab size must be a positive safe integer');
		if (!Number.isFinite(options.typicalHalfwidthCharacterWidth) || options.typicalHalfwidthCharacterWidth <= 0) throw new RangeError('Stanza view-line halfwidth character width must be positive');
		this._typicalHalfwidthCharacterWidth = options.typicalHalfwidthCharacterWidth;
		this.readGpuLineIndexes = options.readGpuLineIndexes ?? (() => EMPTY_LINE_INDEXES);
		this._visibleLines = this._register(new ViewLayer<ViewLine>({
			host: options.host,
			readVisualProjection: options.readVisualProjection,
			readProjectionRevision: options.readProjectionRevision,
			lineRenderer: {
				createLine: visualLineIndex => new ViewLine(this.domNode.domNode, visualLineIndex, this._viewLineOptions, options.tabSize),
				getDomNode: line => line.getDomNode(),
				renderLine: (line, visualLine) => {
						line.getDomNode().dataset.logicalLineIndex = String(visualLine.logicalLineIndex);
						this.projectLineText(line, visualLine, this.resolveSemanticTokensForLine(visualLine));
				},
				layoutLine: (line, lineHeight) => {
					line.layoutLine(lineHeight);
				},
			},
		}));
		this.domNode = new FastDomNode(this._visibleLines.domNode);
		this._register(options.configuration.onDidChange(() => this._onOptionsMaybeChanged(options.configuration, this._context.theme.type)));
	}

	public override dispose(): void {
		super.dispose();
	}

	private _onOptionsMaybeChanged(configuration: IEditorConfiguration, themeType: ColorScheme): boolean {
		const next = new ViewLineOptions(configuration, themeType);
		if (this._viewLineOptions.equals(next)) return false;
		this._viewLineOptions = next;
		const semanticTokens = this.resolveSemanticTokenRange(this._visibleLines.renderedLineRange);
		const visualProjection = this.readVisualProjection();
		for (const [visualLineIndex, line] of this._visibleLines.renderedLines) {
			line.onOptionsChanged(next);
			const visualLine = visualProjection.lineAt(visualLineIndex);
			if (visualLine) this.projectLineText(line, visualLine, semanticTokens.get(visualLine.logicalLineIndex) ?? []);
		}
		this.resetLineWidthCaches();
		return true;
	}

	public getDomNode(): FastDomNode<HTMLDivElement> {
		return this.domNode;
	}

	public renderText(viewportData: ViewportData): void {
		this._visibleLines.render(viewportData);
		this.updateLineWidths();
	}

	public render(context: RestrictedRenderingContext): void {
		this.renderText(context.viewportData);
	}

	public override prepareRender(_context: RestrictedRenderingContext): void {
		this._checkMonospaceFontAssumptions();
	}

	public override onConfigurationChanged(_event?: ViewConfigurationChangedEvent): boolean {
		return this._onOptionsMaybeChanged(this._context.configuration, this._context.theme.type);
	}

	public override onCursorStateChanged(_event?: ViewCursorStateChangedEvent): boolean {
		let changed = false;
		for (const line of this._visibleLines.renderedLines.values()) changed = line.onSelectionChanged() || changed;
		if (changed) this.onTokensChanged();
		return changed;
	}

	public override onDecorationsChanged(_event?: ViewDecorationsChangedEvent): boolean {
		for (const line of this._visibleLines.renderedLines.values()) line.onDecorationsChanged();
		if (this._visibleLines.renderedLines.size === 0) return false;
		this.onTokensChanged();
		return true;
	}

	public override onFlushed(_event?: ViewFlushedEvent): boolean {
		return this.invalidateContent();
	}

	public override onLinesChanged(_event?: ViewLinesChangedEvent): boolean {
		return this.invalidateContent();
	}

	public override onLinesDeleted(_event?: ViewLinesDeletedEvent): boolean {
		return this.invalidateContent();
	}

	public override onLinesInserted(_event?: ViewLinesInsertedEvent): boolean {
		return this.invalidateContent();
	}

	public override onScrollChanged(event: ViewScrollChangedEvent): boolean {
		return event.scrollLeftChanged || event.scrollTopChanged;
	}

	public override onThemeChanged(event: ViewThemeChangedEvent): boolean {
		return this._onOptionsMaybeChanged(this._context.configuration, event.theme.colorScheme);
	}

	public override onZonesChanged(_event?: ViewZonesChangedEvent): boolean {
		return true;
	}

	public getPositionFromDOMInfo(spanNode: HTMLElement, offset: number): Position | null {
		if (!Number.isSafeInteger(offset) || offset < 0) return null;
		const row = this._getViewLineDomNode(spanNode);
		if (!row) return null;
		const lineNumber = this._getLineNumberFor(row);
		const visualLineIndex = lineNumber - 1;
		const line = this._visibleLines.renderedLines.get(visualLineIndex);
		const visualLine = this.readVisualProjection().lineAt(visualLineIndex);
		if (!line || !visualLine) return null;
		const textElement = row.firstElementChild as HTMLElement | null;
		if (!textElement) return null;
		const part = textElement === spanNode
			? textElement.children[offset] ?? textElement.children[offset - 1]
			: directChildOf(textElement, spanNode);
		if (!(part instanceof row.ownerDocument.defaultView!.HTMLElement)) return null;
		const partOffset = textElement === spanNode
			? part === textElement.children[offset] ? 0 : part.textContent?.length ?? 0
			: Math.min(offset, part.textContent?.length ?? 0);
		const column = line.getColumnOfNodeOffset(part, partOffset);
		if (column < 1) return null;
		return new Position((visualLine.logicalLineIndex) + 1, visualLine.startColumn + column);
	}

	private _getViewLineDomNode(node: HTMLElement | null): HTMLElement | null {
		const row = node?.closest(`.${ViewLine.CLASS_NAME}`) as HTMLElement | null;
		return row && row.parentElement === this.domNode.domNode ? row : null;
	}

	private _getLineNumberFor(domNode: HTMLElement): number {
		const visualLineIndex = Number(domNode.dataset.lineIndex);
		if (!Number.isSafeInteger(visualLineIndex) || visualLineIndex < 0) throw new RangeError('View line DOM node has an invalid line index');
		return visualLineIndex + 1;
	}

	public getLineWidth(lineNumber: number): number {
		const line = this._visibleLines.renderedLines.get(lineNumber - 1);
		return line?.getWidth(line ? readingContext(line) : null) ?? this._maxLineWidth;
	}

	public resetLineWidthCaches(): void {
		for (const line of this._visibleLines.renderedLines.values()) line.resetCachedWidth();
		this._maxLineWidth = 0;
	}

	public updateLineWidths(): void {
		this._maxLineWidth = 0;
		for (const line of this._visibleLines.renderedLines.values()) this._ensureMaxLineWidth(line.getWidth(readingContext(line)));
	}

	public linesVisibleRangesForRange(range: Range, includeNewLines: boolean): LineVisibleRanges[] | null {
		this.model.offsetAt(range.getStartPosition());
		this.model.offsetAt(range.getEndPosition());
		const projection = this.readVisualProjection();
		if (projection.modelVersion !== this.model.version) return null;
		const result: LineVisibleRanges[] = [];
		const endVisualLineIndex = projection.visualLineIndexAt(range.getEndPosition());
		const gpuLineIndexes = this.readGpuLineIndexes();
		for (const [visualLineIndex] of this._visibleLines.renderedLines) {
			if (gpuLineIndexes.has(visualLineIndex)) continue;
			const visualLine = projection.lineAt(visualLineIndex);
			if (!visualLine || visualLine.logicalLineIndex < range.startLineNumber - 1 || visualLine.logicalLineIndex > range.endLineNumber - 1) continue;
			const startColumn = visualLine.logicalLineIndex === range.startLineNumber - 1
				? Math.max(visualLine.startColumn, range.startColumn - 1)
				: visualLine.startColumn;
			const endColumn = visualLine.logicalLineIndex === range.endLineNumber - 1
				? Math.min(visualLine.endColumn, range.endColumn - 1)
				: visualLine.endColumn;
			const includesNewLine = includeNewLines && visualLine.lastForLogicalLine && visualLine.logicalLineIndex < range.endLineNumber - 1;
			if (endColumn < startColumn || (endColumn === startColumn && !includesNewLine)) continue;
			const startOffset = startColumn - visualLine.startColumn;
			const endOffset = endColumn - visualLine.startColumn;
			const visibleRanges = this._visibleRangesForLineRange(visualLineIndex + 1, startOffset + 1, endOffset + 1);
			if (!visibleRanges) return null;
			const lineRanges = visibleRanges.ranges.map(range => new FloatHorizontalRange(range.left, range.width));
			if (includesNewLine) {
				const lastRange = lineRanges[lineRanges.length - 1];
				if (!lastRange) return null;
				lastRange.width += this._typicalHalfwidthCharacterWidth;
				if (this._lineIsRenderedRTL(visualLineIndex + 1)) lastRange.left -= this._typicalHalfwidthCharacterWidth;
			}
			result.push(new LineVisibleRanges(
				false,
				visualLineIndex + 1,
				HorizontalRange.from(lineRanges),
				visualLineIndex < endVisualLineIndex,
			));
		}
		return result.length > 0 ? result : null;
	}

	private _visibleRangesForLineRange(lineNumber: number, startColumn: number, endColumn: number): VisibleRanges | null {
		const line = this._visibleLines.renderedLines.get(lineNumber - 1);
		if (!line) return null;
		return line.getVisibleRangesForRange(lineNumber, startColumn, endColumn, readingContext(line));
	}

	private _lineIsRenderedRTL(lineNumber: number): boolean {
		return this._visibleLines.renderedLines.get(lineNumber - 1)?.isRenderedRTL() ?? false;
	}

	public visibleRangeForPosition(position: Position): HorizontalPosition | null {
		this.model.offsetAt(position);
		const projection = this.readVisualProjection();
		if (projection.modelVersion !== this.model.version) return null;
		const visualLineIndex = projection.visualLineIndexAt(position);
		if (this.readGpuLineIndexes().has(visualLineIndex)) return null;
		const visualLine = projection.lineAt(visualLineIndex);
		const renderedLine = this._visibleLines.renderedLines.get(visualLineIndex);
		if (!visualLine || !renderedLine) return null;
		const offset = position.column - 1 - visualLine.startColumn;
		const visibleRanges = this._visibleRangesForLineRange(visualLineIndex + 1, offset + 1, offset + 1);
		const left = visibleRanges?.ranges[0]?.left;
		return left === undefined ? null : new HorizontalPosition(false, left);
	}

	/** Reprojects semantic tokens without rebuilding the visible row window. */
	public override onTokensChanged(_event?: ViewTokensChangedEvent): boolean {
		const semanticTokens = this.resolveSemanticTokenRange(this._visibleLines.renderedLineRange);
		const visualProjection = this.readVisualProjection();
		for (const [visualLineIndex, line] of this._visibleLines.renderedLines) {
			const visualLine = visualProjection.lineAt(visualLineIndex);
			if (visualLine) {
				line.onTokensChanged();
				this.projectLineText(line, visualLine, semanticTokens.get(visualLine.logicalLineIndex) ?? []);
			}
		}
		return true;
	}

	private invalidateContent(): boolean {
		for (const line of this._visibleLines.renderedLines.values()) line.onContentChanged();
		this._maxLineWidth = 0;
		return this._visibleLines.renderedLines.size > 0;
	}

	private _checkMonospaceFontAssumptions(): void {
		let invalid = false;
		for (const line of this._visibleLines.renderedLines.values()) {
			if (!line.needsMonospaceFontCheck() || line.monospaceAssumptionsAreValid()) continue;
			line.onMonospaceAssumptionsInvalidated();
			invalid = true;
		}
		if (invalid) this.onTokensChanged();
	}

	private _ensureMaxLineWidth(lineWidth: number): void {
		if (!Number.isFinite(lineWidth) || lineWidth < 0) throw new RangeError('View line width must be finite and non-negative');
		this._maxLineWidth = Math.max(this._maxLineWidth, lineWidth);
	}

	private resolveSemanticTokensForLine(visualLine: EditorVisualLine): readonly ResolvedSemanticToken[] {
		return this.semanticTokenSource?.getLineTokens(visualLine.logicalLineIndex) ?? [];
	}

	private projectLineText(line: ViewLine, visualLine: EditorVisualLine, tokens: readonly ResolvedSemanticToken[]): void {
		const fullText = this.model.getLineContent((visualLine.logicalLineIndex) + 1);
		const text = fullText.slice(visualLine.startColumn, visualLine.endColumn);
		const brackets = this.bracketColorizationSource?.getLineBrackets(visualLine.logicalLineIndex) ?? [];
		line.renderLine(
			text,
			clipSemanticTokens(tokens, visualLine.startColumn, visualLine.endColumn),
			clipBracketColorizations(brackets, visualLine.startColumn, visualLine.endColumn),
			visualLine.wrappedTextIndentWidth ?? 0,
		);
	}

	private resolveSemanticTokenRange(range: EditorLineRange): ReadonlyMap<number, readonly ResolvedSemanticToken[]> {
		const source = this.semanticTokenSource;
		if (!source) return new Map();
		const tokens = new Map<number, readonly ResolvedSemanticToken[]>();
		const projection = this.readVisualProjection();
		for (let visualLineIndex = range.startLineIndex; visualLineIndex < range.endLineIndexExclusive; visualLineIndex += 1) {
			const visualLine = projection.lineAt(visualLineIndex);
			if (visualLine && !tokens.has(visualLine.logicalLineIndex)) tokens.set(visualLine.logicalLineIndex, source.getLineTokens(visualLine.logicalLineIndex));
		}
		return tokens;
	}
}

function readingContext(line: ViewLine): DomReadingContext {
	const row = line.getDomNode();
	const textElement = row.firstElementChild;
	if (!(textElement instanceof row.ownerDocument.defaultView!.HTMLElement)) throw new Error('Rendered view line has no text element');
	return new DomReadingContext(row, textElement);
}

function directChildOf(parent: HTMLElement, descendant: HTMLElement): HTMLElement | null {
	let current: HTMLElement | null = descendant;
	while (current && current.parentElement !== parent) current = current.parentElement;
	return current?.parentElement === parent ? current : null;
}

const EMPTY_LINE_INDEXES: ReadonlySet<number> = new Set();

function clipSemanticTokens(tokens: readonly ResolvedSemanticToken[], startColumn: number, endColumn: number): readonly ResolvedSemanticToken[] {
	return Object.freeze(tokens.flatMap(token => {
		const start = Math.max(token.startColumn, startColumn);
		const end = Math.min(token.endColumn, endColumn);
		if (end <= start) return [];
		return [Object.freeze({
			startColumn: start - startColumn,
			endColumn: end - startColumn,
			presentation: token.presentation,
			...(token.modifiers && token.modifiers.length > 0 ? { modifiers: token.modifiers } : {}),
			...(token.syntaxPresentation === undefined ? {} : { syntaxPresentation: token.syntaxPresentation }),
		})];
	}));
}
function clipBracketColorizations(brackets: readonly { readonly startColumn: number; readonly endColumn: number; readonly level: number }[], startColumn: number, endColumn: number): readonly { readonly startColumn: number; readonly endColumn: number; readonly level: number }[] {
	return Object.freeze(brackets.flatMap(bracket => {
		const start = Math.max(bracket.startColumn, startColumn);
		const end = Math.min(bracket.endColumn, endColumn);
		if (end <= start) return [];
		return [Object.freeze({ startColumn: start - startColumn, endColumn: end - startColumn, level: bracket.level })];
	}));
}

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
export class LineWidthIndex extends Disposable {
	private widths: number[] = [];
	private readonly widthCounts = new Map<number, number>();
	private readonly changeEmitter = this._register(new Emitter<void>());
	private readonly pendingMeasurement = this._register(new MutableDisposable<IDisposable>());
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
		this.pendingMeasurement.value = options.schedule(() => {
			this.pendingMeasurement.clear();
			if (this.scanVersion !== this.model.version) {
				this.startInitialMeasurement();
				return;
			}
			const previousMaximum = this.maximumWidth;
			this.measureNextSlice(options.linesPerSlice);
			if (this.maximumWidth !== previousMaximum) this.changeEmitter.fire();
			this.scheduleNextSlice();
		});
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
			this.model.getLineContent((lineIndex) + 1),
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
			oldStartLineIndex: contentChange.range.startLineNumber - 1,
			oldEndLineIndex: contentChange.range.endLineNumber - 1,
			lineDelta:
				lineFeedCount(contentChange.text) -
				(
					contentChange.range.getEndPosition().lineNumber -
					contentChange.range.getStartPosition().lineNumber
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
