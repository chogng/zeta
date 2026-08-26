import { addDisposableListener, stopEvent } from "../../../../base/browser/dom.js";
import { DisposableOwner } from "../../../../base/common/lifecycle.js";
import { createToggleBlockCommentCommand } from "../common/blockCommentCommands.js";
import { type EditorSelectionController } from "../../../common/cursor/editorSelectionController.js";
import { type LanguageConfigurationSource } from "../../../common/languages/languageConfiguration.js";
import { type EditorViewport } from "../../../browser/view.js";
import { type LanguageLexicalContextSource } from "../../../common/languages/languageLexicalContext.js";

export interface BlockCommentControllerOptions {
	readonly languageId: string;
	readonly configurations: LanguageConfigurationSource;
	readonly lexicalContext?: LanguageLexicalContextSource;
}

/** Routes the platform block-comment shortcut through Stanza's local command model. */
export class BlockCommentController extends DisposableOwner {
	constructor(
		input: HTMLElement,
		private readonly viewport: EditorViewport,
		private readonly selections: EditorSelectionController,
		private readonly options: BlockCommentControllerOptions,
	) {
		super();
		try {
			validateOptions(options);
			if (viewport.textModel !== selections.textModel) {
				throw new TypeError("Stanza block comment dependencies must share one text model");
			}
			this.own(addDisposableListener(input, "keydown", event => this.handleKeydown(event)));
		} catch (error) {
			this.dispose();
			throw error;
		}
	}

	private handleKeydown(event: KeyboardEvent): void {
		if (event.defaultPrevented || event.isComposing || event.getModifierState("AltGraph")) return;
		if (event.ctrlKey || event.metaKey || !event.shiftKey || !event.altKey || event.key.toLowerCase() !== "a") return;
		const languageId = this.options.lexicalContext?.getLanguageIdAt(this.selections.selections.primary.active) ?? this.options.languageId;
		const blockComment = this.options.configurations.getLanguageConfiguration(languageId).comments.blockComment;
		if (!blockComment) return;
		stopEvent(event);
		this.selections.execute(createToggleBlockCommentCommand(
			this.viewport.textModel,
			this.selections.selections,
			blockComment,
		));
		this.viewport.revealPosition(this.selections.selections.primary.active);
	}
}

function validateOptions(options: BlockCommentControllerOptions): void {
	if (!options || typeof options !== "object" || typeof options.languageId !== "string" || options.languageId.length === 0) {
		throw new TypeError("Stanza block comment controller requires a language ID");
	}
	if (!options.configurations || typeof options.configurations.getLanguageConfiguration !== "function") {
		throw new TypeError("Stanza block comment controller requires language configurations");
	}
}
