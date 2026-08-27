import "./linesDecorations.css";
import { DecorationsPart } from "../decorations/decorationsPart.js";
import { DynamicViewOverlay } from "../../view/dynamicViewOverlay.js";
import { type EditorRenderingContext, EditorViewContext } from "../../view/viewPart.js";
import { projectStanzaLinesDecorations } from "./linesDecorationsProjection.js";
import { type DecorationSource } from "../decorations/decorationPresentation.js";
import { ViewPartRows } from '../../view/viewPartRows.js';

export interface LinesDecorationLaneLayout {
	readonly owner: string;
	readonly left: number;
	readonly width: number;
}

/** Owns line-side decoration classes and tooltips for rendered logical lines. */
export class LinesDecorationsPart extends DynamicViewOverlay {
	public readonly domNode: HTMLElement;
	private readonly decorations: DecorationsPart;
	private readonly lanes: ReadonlyMap<string, LinesDecorationLaneLayout>;
	private readonly rows: ViewPartRows;

	constructor(context: EditorViewContext, host: HTMLElement, decorations: DecorationsPart, sources: readonly DecorationSource[]) {
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
