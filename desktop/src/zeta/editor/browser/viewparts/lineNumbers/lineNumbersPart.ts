import "./lineNumbers.css";
import { DisposableOwner } from "../../../../base/common/lifecycle.js";
import { type EditorVisualLineProjection } from "../../../common/viewModel/modelLineProjection.js";
import { type EditorViewportLayout } from "../../../common/viewLayout/editorViewportModel.js";
import { type RenderedLine } from "../viewLines/renderedLine.js";
import { type EditorViewPart } from "../viewPart.js";

export interface LineNumbersPartOptions {
  readonly showLineNumbers: boolean;
  readonly readVisualProjection: () => EditorVisualLineProjection;
  readonly readRenderedLines: () => ReadonlyMap<number, RenderedLine>;
}

/** Projects line numbers into virtual rows; MarginPart owns feature-gutter slots. */
export class LineNumbersPart extends DisposableOwner implements EditorViewPart {
  private readonly showLineNumbers: boolean;
  private readonly readVisualProjection: () => EditorVisualLineProjection;
  private readonly readRenderedLines: () => ReadonlyMap<number, RenderedLine>;

  constructor(options: LineNumbersPartOptions) {
    super();
    this.showLineNumbers = options.showLineNumbers;
    this.readVisualProjection = options.readVisualProjection;
    this.readRenderedLines = options.readRenderedLines;
  }

  render(_layout: EditorViewportLayout): void {
    const visualProjection = this.readVisualProjection();
    for (const [visualLineIndex, line] of this.readRenderedLines()) {
      const visualLine = visualProjection.lineAt(visualLineIndex);
      if (!visualLine) continue;
      line.numberElement.textContent = this.showLineNumbers && visualLine.firstForLogicalLine
        ? String(visualLine.logicalLineIndex + 1)
        : "";
    }
  }
}
