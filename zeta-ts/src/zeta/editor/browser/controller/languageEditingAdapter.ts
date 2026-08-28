import { Disposable } from "../../../base/common/lifecycle.js";
import { type EditorLanguageEditingAdapter, type EditorLanguageTypeCommand } from "../view/viewController.js";
import { type EditorEditCommand } from "../../common/commands/editorEditCommand.js";
import { type EditorSelectionController } from "../../common/cursor/editorSelectionController.js";
import { LanguageAutoClosingTracker } from "../../common/cursor/languageAutoClosingTracker.js";
import { createLanguageEnterCommand } from "../../common/cursor/languageEnter.js";
import { createLanguagePairBackspaceCommand, createLanguagePairTypeCommand } from "../../common/cursor/languagePairEditing.js";
import { type TextSelectionSet } from "../../common/core/selection.js";
import { type TextModelChange } from "../../common/core/text.js";
import { type LanguageConfigurationSource } from "../../common/languages/languageConfiguration.js";
import { type LanguageLexicalContextSource } from "../../common/languages/languageLexicalContext.js";
import { LanguageLexicalContextIndex } from "../../common/languages/languageLexicalContext.js";
import { assertLanguageId } from "../../common/languages/languageId.js";
import { type TextModel } from "../../common/model/textModel.js";
import { resolveEditorIndentationOptions, type EditorIndentationOptions } from "../../common/editorIndentation.js";

/** Browser input adapter for DOM-free language editing commands. */
export class LanguageEditingAdapter extends Disposable implements EditorLanguageEditingAdapter {
	private readonly autoClosingTracker: LanguageAutoClosingTracker;
	private readonly lexicalContext: LanguageLexicalContextSource;

	constructor(readonly textModel: TextModel, private readonly selections: EditorSelectionController, private readonly languageId: string, private readonly configurations: LanguageConfigurationSource, lexicalContext: LanguageLexicalContextSource | undefined = undefined, private readonly indentation: EditorIndentationOptions | undefined = undefined) {
		super();
		assertLanguageId(languageId);
		if (!configurations || typeof configurations.getLanguageConfiguration !== "function") throw new TypeError("Stanza text input language requires a configuration source");
		resolveEditorIndentationOptions(indentation);
		if (lexicalContext && (lexicalContext.textModel !== textModel || lexicalContext.languageId !== languageId)) throw new TypeError("Stanza text input lexical context must match its model and language");
		this.lexicalContext = lexicalContext ?? this._register(new LanguageLexicalContextIndex(textModel, languageId, configurations));
		this.autoClosingTracker = this._register(new LanguageAutoClosingTracker(textModel, selections));
	}

	createTypeCommand(selections: TextSelectionSet, text: string): EditorLanguageTypeCommand | undefined {
		const result = createLanguagePairTypeCommand(this.textModel, selections, text, this.configurationAt(selections.primary.active), { autoClosingTrust: this.autoClosingTracker, lexicalContext: this.lexicalContext });
		if (!result) return undefined;
		return Object.freeze({
			command: result.command,
			insertedText: result.didInsertText,
			afterExecute: (change: TextModelChange) => {
				if (result.autoClosingActions.length > 0) this.autoClosingTracker.record(result.autoClosingActions, change.version);
			},
		});
	}

	createEnterCommand(selections: TextSelectionSet): EditorEditCommand {
		return createLanguageEnterCommand(this.textModel, selections, this.configurationAt(selections.primary.active), { indentation: this.indentation, lexicalContext: this.lexicalContext });
	}

	createBackspaceCommand(selections: TextSelectionSet): EditorEditCommand | undefined {
		return createLanguagePairBackspaceCommand(this.textModel, selections, this.configurationAt(selections.primary.active), this.autoClosingTracker);
	}

	private configurationAt(position: TextSelectionSet["primary"]["active"]) {
		return this.configurations.getLanguageConfiguration(this.lexicalContext.getLanguageIdAt(position));
	}
}
