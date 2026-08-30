import "./linesDecorations.css";
import { h, reset } from '../../../../base/browser/dom.js';
import { appendIcon } from '../../../../base/browser/ui/icon/icon.js';
import { DecorationsOverlay } from "../decorations/decorations.js";
import { DynamicViewOverlay } from '../../view/dynamicViewOverlay.js';
import { type EditorRenderingContext, EditorViewContext } from "../../view/viewPart.js";
import { type DecorationSource, type ResolvedDecoration } from "../decorations/decorations.js";
import { type EditorOverlayContext } from '../../view/renderingContext.js';
import { ViewPartRows } from '../../view/viewLayer.js';

export interface LinesDecorationLaneLayout {
	readonly owner: string;
	readonly left: number;
	readonly width: number;
}

/** Owns line-side decoration classes and tooltips for rendered logical lines. */
export class LinesDecorationsOverlay extends DynamicViewOverlay {
	public readonly domNode: HTMLElement;
	private readonly decorations: DecorationsOverlay;
	private readonly lanes: ReadonlyMap<string, LinesDecorationLaneLayout>;
	private readonly rows: ViewPartRows;

	constructor(context: EditorViewContext, host: HTMLElement, decorations: DecorationsOverlay, sources: readonly DecorationSource[]) {
		super(context);
		this.rows = this._register(new ViewPartRows(host, 'stanza-editor-lines-decorations-layer', 'stanza-editor-line-lines-decorations'));
		this.domNode = this.rows.domNode;
		this.decorations = decorations;
		this.lanes = new Map(collectLinesDecorationLanes(sources).map(lane => [lane.owner, lane]));
	}

	public render(context: EditorRenderingContext): void {
		const overlay = context.overlay;
		if (!overlay) {
			return;
		}
		projectStanzaLinesDecorations(
			overlay,
			this.decorations.visibleDecorations(overlay),
			this.lanes,
			context.layout.scrollPosition.left,
			this.rows.render(context),
		);
	}
}

/** Projects line-side decoration classes into the currently rendered rows. */
function projectStanzaLinesDecorations(
	context: EditorOverlayContext,
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
		const visualLine = context.visualLineProjection.lineAt(visualLineIndex);
		if (!visualLine || !visualLine.firstForLogicalLine) continue;
		for (const decoration of decorationsByLogicalLine.get(visualLine.logicalLineIndex) ?? []) {
			const presentation = decoration.linesDecoration!;
			const lane = lanes.get(presentation.owner);
			if (!lane) throw new RangeError(`Lines decoration owner '${presentation.owner}' has no layout lane`);
			const classes = [
				presentation.className,
				visualLine.logicalLineIndex === decoration.range.startLineNumber - 1 ? presentation.firstLineClassName : undefined,
			].filter((className): className is string => className !== undefined);
			const element = presentation.icon ? h(context.ownerDocument, 'button') : h(context.ownerDocument, 'div');
			if (presentation.icon) {
				const button = element as HTMLButtonElement;
				button.type = 'button';
				button.setAttribute('aria-label', presentation.ariaLabel!);
				if (presentation.expanded === undefined) button.removeAttribute('aria-expanded');
				else button.setAttribute('aria-expanded', String(presentation.expanded));
			} else {
				element.setAttribute('aria-hidden', 'true');
			}
			element.className = 'stanza-editor-line-decoration';
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
