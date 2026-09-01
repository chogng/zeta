import "./linesDecorations.css";
import { h, reset } from '../../../../base/browser/dom.js';
import { appendIcon } from '../../../../base/browser/ui/icon/icon.js';
import { DecorationsOverlay } from "../decorations/decorations.js";
import { DynamicViewOverlay } from '../../view/dynamicViewOverlay.js';
import { type RenderingContext } from "../../view/renderingContext.js";
import { type ViewContext } from '../../../common/viewModel/viewContext.js';
import { type DecorationSource, type ResolvedDecoration } from "../decorations/decorations.js";
import { type EditorVisualLineProjection } from '../../../common/viewModel/modelLineProjection.js';
import * as viewEvents from '../../../common/viewEvents.js';
import { renderViewPartRows } from '../../view/viewLayer.js';

export interface LinesDecorationLaneLayout {
	readonly owner: string;
	readonly left: number;
	readonly width: number;
}

/** Owns line-side decoration classes and tooltips for rendered logical lines. */
export class LinesDecorationsOverlay extends DynamicViewOverlay {
	private _renderResult: string[] = [];
	private readonly decorations: DecorationsOverlay;
	private readonly lanes: ReadonlyMap<string, LinesDecorationLaneLayout>;

	constructor(private readonly _context: ViewContext, decorations: DecorationsOverlay, sources: readonly DecorationSource[], private readonly ownerDocument: Document, private readonly readVisualProjection: () => EditorVisualLineProjection) {
		super();
		this._context.addEventHandler(this);
		this.decorations = decorations;
		this._register(this.decorations.onDidChange(() => this.forceShouldRender()));
		this.lanes = new Map(collectLinesDecorationLanes(sources).map(lane => [lane.owner, lane]));
	}

	public override dispose(): void {
		this._context.removeEventHandler(this);
		super.dispose();
	}

	public override onConfigurationChanged(_event: viewEvents.ViewConfigurationChangedEvent): boolean { return true; }
	public override onDecorationsChanged(_event: viewEvents.ViewDecorationsChangedEvent): boolean { return true; }
	public override onFlushed(_event: viewEvents.ViewFlushedEvent): boolean { return true; }
	public override onLinesChanged(_event: viewEvents.ViewLinesChangedEvent): boolean { return true; }
	public override onLinesDeleted(_event: viewEvents.ViewLinesDeletedEvent): boolean { return true; }
	public override onLinesInserted(_event: viewEvents.ViewLinesInsertedEvent): boolean { return true; }
	public override onScrollChanged(_event: viewEvents.ViewScrollChangedEvent): boolean { return true; }
	public override onZonesChanged(_event: viewEvents.ViewZonesChangedEvent): boolean { return true; }

	public prepareRender(context: RenderingContext): void {
		this._renderResult = renderViewPartRows(context, this.ownerDocument, rows => projectStanzaLinesDecorations(
			this.readVisualProjection(),
			this._getDecorations(context),
			this.lanes,
			context.scrollLeft,
			rows,
		));
	}

	public render(startLineNumber: number, lineNumber: number): string {
		return this._renderResult[lineNumber - startLineNumber] ?? '';
	}

	private _getDecorations(context: RenderingContext): readonly ResolvedDecoration[] {
		return this.decorations.visibleDecorations(context);
	}
}

/** Projects line-side decoration classes into the currently rendered rows. */
function projectStanzaLinesDecorations(
	projection: EditorVisualLineProjection,
	decorations: readonly ResolvedDecoration[],
	lanes: ReadonlyMap<string, LinesDecorationLaneLayout>,
	scrollLeft: number,
	rows: ReadonlyMap<number, HTMLElement>,
): void {
	const decorationsByLogicalLine = new Map<number, ResolvedDecoration[]>();
	for (const decoration of decorations) {
		if (!decoration.linesDecoration) continue;
		const startLineIndex = decoration.range.startLineNumber - 1;
		const endLineIndex = decoration.range.endColumn === 1 && decoration.range.endLineNumber - 1 > startLineIndex
			? decoration.range.endLineNumber - 2
			: decoration.range.endLineNumber - 1;
		for (let lineIndex = startLineIndex; lineIndex <= endLineIndex; lineIndex += 1) {
			const lineDecorations = decorationsByLogicalLine.get(lineIndex) ?? [];
			lineDecorations.push(decoration);
			decorationsByLogicalLine.set(lineIndex, lineDecorations);
		}
	}

	for (const row of rows.values()) reset(row);
	for (const [visualLineIndex, row] of rows) {
		const visualLine = projection.lineAt(visualLineIndex);
		if (!visualLine || !visualLine.firstForLogicalLine) continue;
		for (const decoration of decorationsByLogicalLine.get(visualLine.logicalLineIndex) ?? []) {
			const presentation = decoration.linesDecoration!;
			const lane = lanes.get(presentation.owner);
			if (!lane) throw new RangeError(`Lines decoration owner '${presentation.owner}' has no layout lane`);
			const classes = [
				presentation.className,
				visualLine.logicalLineIndex === decoration.range.startLineNumber - 1 ? presentation.firstLineClassName : undefined,
			].filter((className): className is string => className !== undefined);
			const element = presentation.icon ? h(row.ownerDocument, 'button') : h(row.ownerDocument, 'div');
			if (presentation.icon) {
				const button = element as HTMLButtonElement;
				button.type = 'button';
				button.setAttribute('aria-label', presentation.ariaLabel!);
				if (presentation.expanded === undefined) button.removeAttribute('aria-expanded');
				else button.setAttribute('aria-expanded', String(presentation.expanded));
			} else {
				element.setAttribute('aria-hidden', 'true');
			}
			element.className = 'cldr stanza-editor-line-decoration';
			for (const className of classes.flatMap(value => value.trim().split(/\s+/u))) element.classList.add(className);
			element.dataset.decorationId = String(decoration.id);
			element.dataset.decorationOwner = presentation.owner;
			element.dataset.logicalLineIndex = String(visualLine.logicalLineIndex);
			element.style.setProperty('--stanza-editor-line-decoration-offset', `${scrollLeft + lane.left}px`);
			element.style.setProperty('--stanza-editor-line-decoration-width', `${lane.width}px`);
			const tooltip = presentation.tooltip ?? decoration.hoverText;
			if (tooltip !== undefined) element.title = tooltip;
			if (presentation.icon) {
				appendIcon(presentation.icon, element);
				element.dataset.iconId = presentation.icon.id;
			}
			row.append(element);
		}
	}
}

export function collectLinesDecorationLanes(sources: readonly DecorationSource[]): readonly LinesDecorationLaneLayout[] {
	const owners = new Set<string>();
	const lanes: LinesDecorationLaneLayout[] = [];
	let left = 0;
	for (const source of sources) {
		for (const definition of source.linesDecorationLanes) {
			if (owners.has(definition.owner)) throw new RangeError(`Duplicate lines decoration owner '${definition.owner}'`);
			owners.add(definition.owner);
			lanes.push(Object.freeze({ owner: definition.owner, left, width: definition.width }));
			left += definition.width;
		}
	}
	return Object.freeze(lanes);
}

export function linesDecorationsWidth(sources: readonly DecorationSource[]): number {
	return collectLinesDecorationLanes(sources).reduce((width, lane) => width + lane.width, 0);
}
