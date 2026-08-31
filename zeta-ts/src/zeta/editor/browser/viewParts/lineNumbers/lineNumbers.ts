import "./lineNumbers.css";
import { h, reset } from "../../../../base/browser/dom.js";
import { type InternalEditorRenderLineNumbersOptions, RenderLineNumbersType } from '../../../common/config/editorOptions.js';
import { type EditorVisualLineProjection } from "../../../common/viewModel/modelLineProjection.js";
import { type IViewModel } from '../../../common/viewModel.js';
import { type RenderingContext } from "../../view/renderingContext.js";
import { type ViewContext } from '../../../common/viewModel/viewContext.js';
import { DynamicViewOverlay } from '../../view/dynamicViewOverlay.js';

interface LineNumbersOverlayOptions {
	readonly lineNumbers: InternalEditorRenderLineNumbersOptions;
	readonly viewModel: IViewModel;
	readonly readVisualProjection: () => EditorVisualLineProjection;
	readonly ownerDocument: Document;
}

/** Projects line numbers into virtual rows. */
export class LineNumbersOverlay extends DynamicViewOverlay {
	public static readonly CLASS_NAME = 'line-numbers';
	private readonly lineNumbers: InternalEditorRenderLineNumbersOptions;
	private readonly viewModel: IViewModel;
	private readonly readVisualProjection: () => EditorVisualLineProjection;
	private readonly ownerDocument: Document;

	constructor(private readonly context: ViewContext, options: LineNumbersOverlayOptions) {
		super();
		this.context.addEventHandler(this);
		this.lineNumbers = options.lineNumbers;
		this.viewModel = options.viewModel;
		this.readVisualProjection = options.readVisualProjection;
		this.ownerDocument = options.ownerDocument;
	}

	public override dispose(): void {
		this.context.removeEventHandler(this);
		super.dispose();
	}

	public prepareRender(context: RenderingContext): void {
		const visualProjection = this.readVisualProjection();
		const activeLineIndex = this.viewModel.getPrimaryCursorState().modelState.position.lineNumber - 1;
		this.prepareRows(context, this.ownerDocument, rows => {
		for (const [visualLineIndex, row] of rows) {
			const visualLine = visualProjection.lineAt(visualLineIndex);
			if (!visualLine) continue;
			const number = h(row.ownerDocument, "span");
			number.className = LineNumbersOverlay.CLASS_NAME;
			number.classList.toggle("active", visualLine.logicalLineIndex === activeLineIndex);
			number.classList.toggle("active-line-number", visualLine.logicalLineIndex === activeLineIndex);
			number.textContent = visualLine.firstForLogicalLine
				? renderLineNumber(this.lineNumbers, visualLine.logicalLineIndex, activeLineIndex)
				: '';
			reset(row, number);
		}
		});
	}
}

function renderLineNumber(options: InternalEditorRenderLineNumbersOptions, lineIndex: number, activeLineIndex: number | undefined): string {
	const lineNumber = lineIndex + 1;
	switch (options.renderType) {
		case RenderLineNumbersType.Off: return '';
		case RenderLineNumbersType.On: return String(lineNumber);
		case RenderLineNumbersType.Relative:
			return activeLineIndex === undefined || activeLineIndex === lineIndex ? String(lineNumber) : String(Math.abs(lineIndex - activeLineIndex));
		case RenderLineNumbersType.Interval: return lineNumber % 10 === 0 ? String(lineNumber) : '';
		case RenderLineNumbersType.Custom: return options.renderFn?.(lineNumber) ?? '';
	}
}
