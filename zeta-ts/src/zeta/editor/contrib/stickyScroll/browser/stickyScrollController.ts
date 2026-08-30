import "./media/stickyScroll.css";
import { Disposable, toDisposable } from "../../../../base/common/lifecycle.js";
import { Position } from "../../../common/core/position.js";
import { type View } from "../../../browser/view.js";
import { type EditorFoldingModel } from "../../folding/browser/foldingModel.js";
import { buildStickyScrollEntries } from "../common/stickyScrollModel.js";
import { h } from "../../../../base/browser/dom.js";

/** Projects folding ancestors above the viewport as an accessible sticky header stack. */
export class StickyScrollController extends Disposable {
	private readonly element: HTMLDivElement;

	constructor(private readonly viewport: View, private readonly folding: EditorFoldingModel) {
		super();
		if (folding.model !== viewport.textModel) throw new TypeError("Stanza sticky scroll dependencies must share a text model");
		this.element = h(viewport.element.ownerDocument, "div");
		this.element.className = "stanza-editor-sticky-scroll";
		this.element.setAttribute("aria-label", "Sticky section headers");
		viewport.element.append(this.element);
		this._register(toDisposable(() => this.element.remove()));
		this._register(viewport.onDidChangeLayout(() => this.render()));
		this._register(folding.onDidChange(() => this.render()));
		this.render();
	}

	private render(): void {
		const visual = this.viewport.getVisualLineProjection();
		const firstVisualLine = this.viewport.viewportLayout.visibleLines.startLineIndex;
		const first = visual.lineAt(firstVisualLine);
		if (!first) { this.element.hidden = true; return; }
		const entries = buildStickyScrollEntries(this.viewport.textModel, first.logicalLineIndex, this.folding.regions);
		this.element.replaceChildren(...entries.map(entry => {
			const button = h(this.element.ownerDocument, "button");
			button.type = "button";
			button.className = "stanza-editor-sticky-scroll-item";
			button.style.paddingLeft = `${8 + entry.depth * 12}px`;
			button.textContent = entry.label || `Line ${entry.lineIndex + 1}`;
			button.title = `Reveal line ${entry.lineIndex + 1}`;
			button.addEventListener("click", () => this.viewport.revealPosition(new Position((entry.lineIndex) + 1, (0) + 1)));
			return button;
		}));
		this.element.hidden = entries.length === 0;
	}
}
