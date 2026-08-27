import { toDisposable } from "../../../../base/common/lifecycle.js";
import "./composition.css";
import { type TextRange } from "../../../common/core/text.js";
import { type TextModel } from "../../../common/model/textModel.js";
import { TrackedRangeStickiness, type TrackedRange } from "../../../common/model/trackedRange.js";
import { projectStanzaCompositionOverlay } from "./compositionProjection.js";
import { DynamicViewOverlay } from "../../view/dynamicViewOverlay.js";
import { type EditorRenderingContext, EditorViewContext } from "../../view/viewPart.js";

/** Owns tracked IME range presentation while EditorView owns composition state. */
export class CompositionPart extends DynamicViewOverlay {
	private readonly model: TextModel;
	private compositionRange: TrackedRange | undefined;

	constructor(context: EditorViewContext, model: TextModel) {
		super(context);
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
		projectStanzaCompositionOverlay(overlay, this.compositionRange?.range);
	}
}
