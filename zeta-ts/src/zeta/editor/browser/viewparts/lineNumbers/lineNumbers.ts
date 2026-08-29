import "./lineNumbers.css";
import { h, reset } from "../../../../base/browser/dom.js";
import { type EditorSelectionController } from "../../../common/cursor/editorSelectionController.js";
import { type EditorVisualLineProjection } from "../../../common/viewModel/modelLineProjection.js";
import { EditorViewPart, type EditorRenderingContext } from "../../view/viewPart.js";
import { ViewPartRows } from "../../view/viewLayer.js";

export interface LineNumbersOverlayOptions {
	readonly host: HTMLElement;
	readonly showLineNumbers: boolean;
	readonly selectionController: EditorSelectionController | undefined;
	readonly readVisualProjection: () => EditorVisualLineProjection;
}

/** Projects line numbers into virtual rows. */
export class LineNumbersOverlay extends EditorViewPart {
	public readonly domNode: HTMLElement;
	private readonly showLineNumbers: boolean;
	private readonly selectionController: EditorSelectionController | undefined;
	private readonly readVisualProjection: () => EditorVisualLineProjection;
	private readonly rows: ViewPartRows;

	constructor(options: LineNumbersOverlayOptions) {
		super();
		this.rows = this._register(new ViewPartRows(options.host, "stanza-editor-line-numbers-layer", "stanza-editor-line-margin"));
		this.domNode = this.rows.domNode;
		this.showLineNumbers = options.showLineNumbers;
		this.selectionController = options.selectionController;
		this.readVisualProjection = options.readVisualProjection;
	}

	render(context: EditorRenderingContext): void {
		this.domNode.style.left = `${context.layout.scrollPosition.left}px`;
		const visualProjection = this.readVisualProjection();
		const activeLineIndex = this.selectionController?.selections.primary.active.lineIndex;
		for (const [visualLineIndex, row] of this.rows.render(context)) {
			const visualLine = visualProjection.lineAt(visualLineIndex);
			if (!visualLine) continue;
			const number = row.firstElementChild as HTMLElement | null ?? h(row.ownerDocument, "span");
			number.className = "stanza-editor-line-number";
			number.classList.toggle("active", visualLine.logicalLineIndex === activeLineIndex);
			number.textContent = this.showLineNumbers && visualLine.firstForLogicalLine
				? String(visualLine.logicalLineIndex + 1)
				: "";
			reset(row, number);
		}
	}
}
