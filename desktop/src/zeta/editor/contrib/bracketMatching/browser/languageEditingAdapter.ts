import { DisposableOwner } from "../../../../base/common/lifecycle.js";
import { registerTextInputLanguageEditingFactory, type TextInputIndentationOptions, type TextInputLanguageEditingAdapter, type TextInputLanguageOptions, type TextInputLanguageTypeCommand } from "../../../browser/input/textInputController.js";
import { type EditorEditCommand } from "../../../common/commands/editorEditCommand.js";
import { type EditorSelectionController } from "../../../common/cursor/editorSelectionController.js";
import { type TextSelectionSet } from "../../../common/core/selection.js";
import { type TextModelChange } from "../../../common/core/text.js";
import { type LanguageConfigurationSource } from "../../../common/languages/languageConfiguration.js";
import { type LanguageLexicalContextSource } from "../../../common/languages/languageLexicalContext.js";
import { LanguageLexicalContextIndex } from "../../../common/languages/languageLexicalContext.js";
import { assertLanguageId } from "../../../common/languages/languageId.js";
import { type TextModel } from "../../../common/model/textModel.js";
import { resolveEditorIndentationOptions, type EditorIndentationOptions } from "../../../common/editorIndentation.js";
import { LanguageAutoClosingTracker } from "../common/autoClosingTracker.js";
import { createLanguageEnterCommand } from "../common/enter.js";
import { createLanguagePairBackspaceCommand, createLanguagePairTypeCommand } from "../common/pairEditing.js";

/** Language-aware typing adapter selected by the bracket-matching contribution. */
export class LanguageEditingAdapter extends DisposableOwner implements TextInputLanguageEditingAdapter {
  private readonly autoClosingTracker: LanguageAutoClosingTracker;
  private readonly lexicalContext: LanguageLexicalContextSource;

  constructor(readonly textModel: TextModel, private readonly selections: EditorSelectionController, private readonly languageId: string, private readonly configurations: LanguageConfigurationSource, lexicalContext: LanguageLexicalContextSource | undefined, private readonly indentation: EditorIndentationOptions | undefined) {
    super();
    assertLanguageId(languageId);
    if (!configurations || typeof configurations.getLanguageConfiguration !== "function") throw new TypeError("Aster text input language requires a configuration source");
    resolveEditorIndentationOptions(indentation);
    if (lexicalContext && (lexicalContext.textModel !== textModel || lexicalContext.languageId !== languageId)) throw new TypeError("Aster text input lexical context must match its model and language");
    this.lexicalContext = lexicalContext ?? this.own(new LanguageLexicalContextIndex(textModel, languageId, configurations));
    this.autoClosingTracker = this.own(new LanguageAutoClosingTracker(textModel, selections));
  }

  createTypeCommand(selections: TextSelectionSet, text: string): TextInputLanguageTypeCommand | undefined {
    const result = createLanguagePairTypeCommand(this.textModel, selections, text, this.configuration, { autoClosingTrust: this.autoClosingTracker, lexicalContext: this.lexicalContext });
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
    return createLanguageEnterCommand(this.textModel, selections, this.configuration, { indentation: this.indentation, lexicalContext: this.lexicalContext });
  }

  createBackspaceCommand(selections: TextSelectionSet): EditorEditCommand | undefined {
    return createLanguagePairBackspaceCommand(this.textModel, selections, this.configuration, this.autoClosingTracker);
  }

  private get configuration() {
    return this.configurations.getLanguageConfiguration(this.languageId);
  }
}

registerTextInputLanguageEditingFactory((model, selections, language: TextInputLanguageOptions, indentation: TextInputIndentationOptions | undefined) => new LanguageEditingAdapter(model, selections, language.languageId, language.configurations, language.lexicalContext, indentation as EditorIndentationOptions | undefined));
