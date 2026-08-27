import "./lineNumbers.css";
import { type EditorVisualLineProjection } from "../../../common/viewModel/modelLineProjection.js";
import { type ViewLine } from "../viewLines/viewLine.js";
import { EditorViewPart, type EditorRenderingContext } from "../../view/viewPart.js";

export interface LineNumbersPartOptions {
	readonly showLineNumbers: boolean;
	readonly readVisualProjection: () => EditorVisualLineProjection;
	readonly readRenderedLines: () => ReadonlyMap<number, ViewLine>;
}

/** Projects line numbers into virtual rows. */
export class LineNumbersPart extends EditorViewPart {
	private readonly showLineNumbers: boolean;
	private readonly readVisualProjection: () => EditorVisualLineProjection;
	private readonly readRenderedLines: () => ReadonlyMap<number, ViewLine>;

	constructor(options: LineNumbersPartOptions) {
		super();
		this.showLineNumbers = options.showLineNumbers;
		this.readVisualProjection = options.readVisualProjection;
		this.readRenderedLines = options.readRenderedLines;
	}

	render(_context: EditorRenderingContext): void {
		const visualProjection = this.readVisualProjection();
		for (const [visualLineIndex, line] of this.readRenderedLines()) {
			const visualLine = visualProjection.lineAt(visualLineIndex);
			if (!visualLine) continue;
			line.numberDomNode.setTextContent(this.showLineNumbers && visualLine.firstForLogicalLine
				? String(visualLine.logicalLineIndex + 1)
				: "");
		}
	}
}
