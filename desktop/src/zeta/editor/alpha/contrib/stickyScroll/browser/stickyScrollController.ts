import "./media/stickyScroll.css";
import { DisposableOwner } from "../../../../../base/common/lifecycle.js";
import { TextPosition } from "../../../common/core/text.js";
import { type AlphaEditorViewport } from "../../../browser/view/editorViewport.js";
import { type EditorFoldingModel } from "../../folding/browser/foldingModel.js";
import { buildStickyScrollEntries } from "../common/stickyScrollModel.js";

/** Projects folding ancestors above the viewport as an accessible sticky header stack. */
export class AlphaStickyScrollController extends DisposableOwner {
  private readonly element: HTMLDivElement;

  constructor(private readonly viewport: AlphaEditorViewport, private readonly folding: EditorFoldingModel) {
    super();
    if (folding.model !== viewport.textModel) throw new TypeError("Alpha sticky scroll dependencies must share a text model");
    this.element = viewport.element.ownerDocument.createElement("div");
    this.element.className = "zeta-alpha-editor-sticky-scroll";
    this.element.setAttribute("aria-label", "Sticky section headers");
    viewport.element.append(this.element);
    this.defer(() => this.element.remove());
    this.own(viewport.onDidChangeLayout(() => this.render()));
    this.own(folding.onDidChange(() => this.render()));
    this.render();
  }

  private render(): void {
    const visual = this.viewport.getVisualLineProjection();
    const firstVisualLine = this.viewport.viewportLayout.visibleLines.startLineIndex;
    const first = visual.lineAt(firstVisualLine);
    if (!first) { this.element.hidden = true; return; }
    const entries = buildStickyScrollEntries(this.viewport.textModel, first.logicalLineIndex, this.folding.regions);
    this.element.replaceChildren(...entries.map(entry => {
      const button = this.element.ownerDocument.createElement("button");
      button.type = "button";
      button.className = "zeta-alpha-editor-sticky-scroll-item";
      button.style.paddingLeft = `${8 + entry.depth * 12}px`;
      button.textContent = entry.label || `Line ${entry.lineIndex + 1}`;
      button.title = `Reveal line ${entry.lineIndex + 1}`;
      button.addEventListener("click", () => this.viewport.revealPosition(TextPosition.at(entry.lineIndex, 0)));
      return button;
    }));
    this.element.hidden = entries.length === 0;
  }
}
