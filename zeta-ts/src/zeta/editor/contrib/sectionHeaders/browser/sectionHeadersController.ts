import "./media/sectionHeaders.css";
import { Disposable } from "../../../../base/common/lifecycle.js";
import { type LanguageConfigurationSource } from "../../../common/languages/ownedLanguageConfigurationContributions.js";
import { type LanguageLexicalContextSource } from "../../../common/languages/languageLexicalContext.js";
import { findSectionHeaders, type FindSectionHeaderOptions } from "../../../common/services/findSectionHeaders.js";
import { type TextModel } from "../../../common/model/textModel.js";
import { Position } from '../../../common/core/position.js';
import { type EditorViewport } from "../../../browser/view.js";

/** Marks named source sections for browser presentation and accessibility. */
export class SectionHeadersController extends Disposable {
	constructor(
		private readonly viewport: EditorViewport,
		private readonly model: TextModel,
		private readonly languageId: string,
		private readonly configurations: LanguageConfigurationSource,
		private readonly lexicalContext: LanguageLexicalContextSource,
		private readonly options: Omit<FindSectionHeaderOptions, "foldingRules">,
	) {
		super();
		if (model !== viewport.textModel || lexicalContext.textModel !== model) throw new TypeError("Stanza section header dependencies must share a text model");
		this._register(viewport.onDidChangeLayout(() => this.update()));
		this._register(model.onDidChange(() => this.update()));
		this.update();
	}

	private update(): void {
		const configuration = this.configurations.getLanguageConfiguration(this.languageId);
		const headers = new Map(findSectionHeaders(this.model, {
			...this.options,
			foldingRules: configuration.foldingMarkers ? { markers: configuration.foldingMarkers } : undefined,
		}).filter(header => !header.shouldBeInComments || this.lexicalContext.getTokenTypeAt(new Position(header.range.startLineNumber, header.range.startColumn)) === "comment")
			.map(header => [header.range.startLineNumber - 1, header]));
		for (const line of [...this.viewport.element.querySelectorAll<HTMLElement>(".view-line")]) {
			const logicalLineIndex = Number(line.dataset.logicalLineIndex);
			const header = headers.get(logicalLineIndex);
			line.classList.toggle("section-header", Boolean(header));
			if (header) {
				line.setAttribute("data-section-header", "true");
				line.classList.toggle("section-header-separator", header.hasSeparatorLine);
			} else {
				line.removeAttribute("data-section-header");
				line.classList.remove("section-header-separator");
			}
		}
	}
}
