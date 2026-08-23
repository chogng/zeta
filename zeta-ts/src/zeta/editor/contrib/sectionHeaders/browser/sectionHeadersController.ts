import "./media/sectionHeaders.css";
import { DisposableOwner } from "../../../../base/common/lifecycle.js";
import { type EditorViewport } from "../../../browser/view/editorViewport.js";
import { type EditorFoldingModel } from "../../folding/browser/foldingModel.js";

/** Marks logical lines that introduce a foldable section for browser presentation and accessibility. */
export class SectionHeadersController extends DisposableOwner {
	constructor(private readonly viewport: EditorViewport, private readonly folding: EditorFoldingModel) {
		super();
		if (folding.model !== viewport.textModel) throw new TypeError("Stanza section header dependencies must share a text model");
		this.own(viewport.onDidChangeLayout(() => this.update()));
		this.own(folding.onDidChange(() => this.update()));
		this.own(viewport.textModel.onDidChange(() => this.update()));
		this.update();
	}

	private update(): void {
		const headers = new Set(this.folding.regions.map(region => region.startLineIndex));
		for (const line of [...this.viewport.element.querySelectorAll<HTMLElement>(".stanza-editor-line")]) {
			const logicalLineIndex = Number(line.dataset.logicalLineIndex);
			line.classList.toggle("section-header", headers.has(logicalLineIndex));
			if (headers.has(logicalLineIndex)) line.setAttribute("data-section-header", "true");
			else line.removeAttribute("data-section-header");
		}
	}
}
