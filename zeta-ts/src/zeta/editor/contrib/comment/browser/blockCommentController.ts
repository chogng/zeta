import { addDisposableListener, stopEvent } from "../../../../base/browser/dom.js";
import { Disposable } from "../../../../base/common/lifecycle.js";
import { createToggleBlockCommentCommand } from "../common/blockCommentCommands.js";
import { type CursorsController } from "../../../common/cursor/cursor.js";
import { type LanguageConfigurationSource } from "../../../common/languages/ownedLanguageConfigurationContributions.js";
import { type EditorViewport } from "../../../browser/view.js";
import { type LanguageLexicalContextSource } from "../../../common/languages/languageLexicalContext.js";
import { type EditorCommandExecutor } from '../../../browser/editorExtensions.js';

export const ToggleBlockCommentCommandId = 'editor.action.blockComment';

export interface BlockCommentControllerOptions {
	readonly languageId: string;
	readonly configurations: LanguageConfigurationSource;
	readonly lexicalContext?: LanguageLexicalContextSource;
}

/** Routes the platform block-comment shortcut through Stanza's local command model. */
export class BlockCommentController extends Disposable {
	constructor(
		input: HTMLElement,
		private readonly viewport: EditorViewport,
		private readonly selections: CursorsController,
		private readonly options: BlockCommentControllerOptions,
		private readonly executeCommand: EditorCommandExecutor = (_commandId, operation) => operation(),
	) {
		super();
		try {
			validateOptions(options);
			if (viewport.textModel !== selections.textModel) {
				throw new TypeError("Stanza block comment dependencies must share one text model");
			}
			this._register(addDisposableListener(input, "keydown", event => this.handleKeydown(event)));
		} catch (error) {
			this.dispose();
			throw error;
		}
	}

	private handleKeydown(event: KeyboardEvent): void {
		if (event.defaultPrevented || event.isComposing || event.getModifierState("AltGraph")) return;
		if (event.ctrlKey || event.metaKey || !event.shiftKey || !event.altKey || event.key.toLowerCase() !== "a") return;
		const languageId = this.options.lexicalContext?.getLanguageIdAt(this.selections.selections.primary.getPosition()) ?? this.options.languageId;
		const blockComment = this.options.configurations.getLanguageConfiguration(languageId).comments.blockComment;
		if (!blockComment) return;
		stopEvent(event);
		const command = createToggleBlockCommentCommand(
			this.viewport.textModel,
			this.selections.selections,
			blockComment,
		);
		this.executeCommand(ToggleBlockCommentCommandId, () => this.selections.execute(command));
		this.viewport.revealPosition(this.selections.selections.primary.getPosition());
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
