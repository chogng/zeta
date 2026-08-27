import { toDisposable } from "../../../../base/common/lifecycle.js";
import "./composition.css";
import { type TextRange } from "../../../common/core/text.js";
import { type TextModel } from "../../../common/model/textModel.js";
import { TrackedRangeStickiness, type TrackedRange } from "../../../common/model/trackedRange.js";
import { projectStanzaCompositionOverlay } from "./compositionProjection.js";
import { DynamicViewOverlay } from "../../view/dynamicViewOverlay.js";
import { type EditorRenderingContext, EditorViewContext } from "../../view/viewPart.js";
import { ViewPartRows } from '../../view/viewPartRows.js';

/** Owns tracked IME range presentation while EditorView owns composition state. */
export class CompositionPart extends DynamicViewOverlay {
	public readonly domNode: HTMLElement;
	private readonly model: TextModel;
	private readonly rows: ViewPartRows;
	private compositionRange: TrackedRange | undefined;

	constructor(context: EditorViewContext, host: HTMLElement, model: TextModel) {
		super(context);
		this.rows = this._register(new ViewPartRows(host, 'stanza-editor-composition-layer', 'stanza-editor-line-composition'));
		this.domNode = this.rows.domNode;
		this.model = model;
		this._register(toDisposable(() => this.compositionRange?.dispose()));
	}

	public setRange(range: TextRange | undefined): void {
		const next = range
			? this.model.trackRange(range, TrackedRangeStickiness.NeverGrowsAtEdges)
			: undefined;
		this.compositionRange?.dispose();
		this.compositionRange = next;
		this.renderNow(this.context.renderingContext);
	}

	public render(context: EditorRenderingContext): void {
		const overlay = context.overlay;
		if (!overlay) {
			return;
		}
		projectStanzaCompositionOverlay(overlay, this.compositionRange?.range, this.rows.render(context));
	}
}
