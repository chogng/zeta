import { addDisposableListener, stopEvent } from "../../../../base/browser/dom.js";
import { Disposable } from "../../../../base/common/lifecycle.js";
import { type CursorsController } from "../../../common/cursor/cursor.js";
import { createToggleLineCommentCommand } from "../common/lineCommentCommands.js";
import { type ILanguageConfigurationService } from '../../../common/languages/languageConfigurationRegistry.js';
import { type View } from "../../../browser/view.js";
import { type LanguageLexicalContextSource } from "../../../common/languages/languageLexicalContext.js";
import { type EditorCommandExecutor } from '../../../browser/editorExtensions.js';

export const ToggleLineCommentCommandId = 'editor.action.commentLine';

export interface LineCommentControllerOptions {
	readonly languageId: string;
	readonly configurations: ILanguageConfigurationService;
	readonly insertSpace?: boolean;
	readonly lexicalContext?: LanguageLexicalContextSource;
}

/** Routes the platform line-comment shortcut through Stanza's local command model. */
export class LineCommentController extends Disposable {
	constructor(
		input: HTMLElement,
		private readonly viewport: View,
		private readonly selections: CursorsController,
		private readonly options: LineCommentControllerOptions,
		private readonly executeCommand: EditorCommandExecutor = (_commandId, operation) => operation(),
	) {
		super();
		try {
			validateOptions(options);
			if (viewport.textModel !== selections.textModel) {
				throw new TypeError("Stanza line comment dependencies must share one text model");
			}
			this._register(addDisposableListener(input, "keydown", event => this.handleKeydown(event)));
		} catch (error) {
			this.dispose();
			throw error;
		}
	}

	private handleKeydown(event: KeyboardEvent): void {
		if (event.defaultPrevented || event.isComposing || event.getModifierState("AltGraph")) return;
		if ((!event.ctrlKey && !event.metaKey) || event.shiftKey || event.altKey || event.key !== "/") return;
		const languageId = this.options.lexicalContext?.getLanguageIdAt(this.selections.selections[0]!.getPosition()) ?? this.options.languageId;
		const lineComment = this.options.configurations.getLanguageConfiguration(languageId).comments?.lineCommentToken;
		if (!lineComment) return;
		stopEvent(event);
		const command = createToggleLineCommentCommand(
			this.viewport.textModel,
			this.selections.selections,
			{
				lineComment,
				insertSpace: this.options.insertSpace,
			},
		);
		this.executeCommand(ToggleLineCommentCommandId, () => this.selections.execute(command));
		this.viewport.revealPosition(this.selections.selections[0]!.getPosition());
	}
}

function validateOptions(options: LineCommentControllerOptions): void {
	if (!options || typeof options !== "object" || typeof options.languageId !== "string" || options.languageId.length === 0) {
		throw new TypeError("Stanza line comment controller requires a language ID");
	}
	if (!options.configurations || typeof options.configurations.getLanguageConfiguration !== "function") {
		throw new TypeError("Stanza line comment controller requires language configurations");
	}
	if (options.insertSpace !== undefined && typeof options.insertSpace !== "boolean") {
		throw new TypeError("Stanza line comment insertSpace must be a boolean");
	}
}
